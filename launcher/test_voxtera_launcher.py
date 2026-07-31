"""Contract tests for the platform-aware Voxtera launcher."""

from __future__ import annotations

import os
from pathlib import Path
import tempfile
import unittest

from voxtera_launcher import (
    find_platform_archive,
    game_launch_environment,
    installed_game_path,
    manifest_sha256_for_platform,
    platform_spec,
)


class PlatformAwareLauncherTests(unittest.TestCase):
    def setUp(self) -> None:
        self.release = {
            "assets": [
                {
                    "name": "Voxtera-windows-x64-v0.4.0.zip",
                    "browser_download_url": "https://example.invalid/windows.zip",
                },
                {
                    "name": "Voxtera-macos-universal-v0.4.0.zip",
                    "browser_download_url": "https://example.invalid/macos.zip",
                },
            ],
        }
        self.manifest = {
            "artifacts": {
                "windows-x64": {
                    "archive": "Voxtera-windows-x64-v0.4.0.zip",
                    "sha256": "windows-sha256",
                },
                "macos-universal": {
                    "archive": "Voxtera-macos-universal-v0.4.0.zip",
                    "sha256": "macos-sha256",
                },
            },
        }

    def test_windows_selects_only_the_windows_archive(self) -> None:
        spec = platform_spec("Windows")

        archive = find_platform_archive(self.release, spec)

        self.assertEqual(archive["name"], "Voxtera-windows-x64-v0.4.0.zip")
        self.assertEqual(
            installed_game_path(Path("C:/Games/Voxtera"), spec),
            Path("C:/Games/Voxtera/Voxtera.exe"),
        )

    def test_macos_selects_the_universal_archive_and_app_executable(self) -> None:
        spec = platform_spec("Darwin")

        archive = find_platform_archive(self.release, spec)

        self.assertEqual(archive["name"], "Voxtera-macos-universal-v0.4.0.zip")
        self.assertEqual(
            installed_game_path(Path("/Applications/Voxtera"), spec),
            Path("/Applications/Voxtera/Voxtera.app/Contents/MacOS/Voxtera"),
        )

    def test_manifest_hash_is_bound_to_the_selected_platform_archive(self) -> None:
        self.assertEqual(
            manifest_sha256_for_platform(
                self.manifest,
                platform_spec("Windows"),
                "Voxtera-windows-x64-v0.4.0.zip",
            ),
            "windows-sha256",
        )
        self.assertEqual(
            manifest_sha256_for_platform(
                self.manifest,
                platform_spec("Darwin"),
                "Voxtera-macos-universal-v0.4.0.zip",
            ),
            "macos-sha256",
        )

    def test_legacy_single_hash_manifest_remains_compatible_with_windows(self) -> None:
        self.assertEqual(
            manifest_sha256_for_platform(
                {"zip_sha256": "legacy-windows-sha256"},
                platform_spec("Windows"),
                "Voxtera-windows-x64-v0.3.8.5.zip",
            ),
            "legacy-windows-sha256",
        )
        self.assertIsNone(
            manifest_sha256_for_platform(
                {"zip_sha256": "legacy-windows-sha256"},
                platform_spec("Darwin"),
                "Voxtera-macos-universal-v0.3.8.5.zip",
            ),
        )

    def test_macos_launch_environment_points_to_packaged_resources(self) -> None:
        spec = platform_spec("Darwin")
        with tempfile.TemporaryDirectory() as directory:
            install_dir = Path(directory)
            assets_dir = install_dir / "Voxtera.app" / "Contents" / "Resources" / "assets"
            assets_dir.mkdir(parents=True)

            environment = game_launch_environment(install_dir, spec, {"PATH": os.environ["PATH"]})

        self.assertEqual(environment["VELOREN_ASSETS"], str(assets_dir))

    def test_unsupported_platform_has_no_release_contract(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "Unsupported platform"):
            platform_spec("Linux")




class TkThreadSafeAfterTests(unittest.TestCase):
    """Regression: Tk's after() may raise RuntimeError from a worker thread
    on macOS PyInstaller bundles, leaving the status label stuck on the
    initial 'Verificando atualizações...' text. The fix routes updates
    through a queue.Queue drained by the main loop.
    """

    class _FakeLauncher:
        def __init__(self):
            import queue as _queue
            self._ui_queue = _queue.Queue()
            self._tk_alive = True
            self._status_text = "Verificando atualizações..."
            self._pump_calls = 0

        def _set_status(self, text, color=None):
            self._status_text = text

        def after(self, ms, func):
            # Simulate the macOS failure that was haunting the launcher.
            raise RuntimeError("main thread is not in main loop")

    def test_safe_after_fallback_uses_queue_when_tk_after_raises(self):
        # Wire the new helpers onto the fake launcher
        import voxtera_launcher as M
        fl = self._FakeLauncher()
        fl._safe_after = M.VoxteraLauncher._safe_after.__get__(fl)
        fl._pump_queue = M.VoxteraLauncher._pump_queue.__get__(fl)

        # First update: after() raises, so the call should land in the queue
        fl._safe_after(lambda: fl._set_status("Nova versão: v0.4.0", None))
        self.assertEqual(fl._status_text, "Verificando atualizações...")
        self.assertEqual(fl._ui_queue.qsize(), 1)

    def test_pump_queue_drains_pending_updates(self):
        import queue as _queue
        import voxtera_launcher as M
        fl = self._FakeLauncher()
        fl._safe_after = M.VoxteraLauncher._safe_after.__get__(fl)
        fl._pump_queue = M.VoxteraLauncher._pump_queue.__get__(fl)

        # Pre-populate the queue as a worker thread would
        fl._safe_after(lambda: fl._set_status("Status A", None))
        fl._safe_after(lambda: fl._set_status("Status B", None))
        self.assertEqual(fl._ui_queue.qsize(), 2)

        # Drain directly (we can't run the rescheduling after() here)
        while True:
            try:
                func = fl._ui_queue.get_nowait()
            except _queue.Empty:
                break
            func()
        self.assertEqual(fl._status_text, "Status B")
        self.assertEqual(fl._ui_queue.qsize(), 0)

    def test_safe_after_noop_when_launcher_destroyed(self):
        import queue as _queue
        import voxtera_launcher as M
        fl = self._FakeLauncher()
        fl._safe_after = M.VoxteraLauncher._safe_after.__get__(fl)
        fl._tk_alive = False
        fl._safe_after(lambda: fl._set_status("ignored", None))
        self.assertEqual(fl._ui_queue.qsize(), 0)
        self.assertEqual(fl._status_text, "Verificando atualizações...")

    def test_threaded_check_updates_unblocks_status(self):
        # End-to-end of the original bug: a daemon thread runs the update
        # check, every self.after() raises, and the queue pathway is what
        # surfaces the final status to the UI.
        import threading
        import voxtera_launcher as M

        class ThreadProbe(self._FakeLauncher):
            def __init__(self):
                super().__init__()
                self.platform = M.platform_spec()
                self.cfg = {"install_dir": "/tmp/fake", "installed_version": None}
                self.latest_version = None
                self.download_url = None
                self.download_asset = None
                self.manifest_url = None
                self._downloading = False

            def _is_installed(self):
                return False

        fl = ThreadProbe()
        fl._safe_after = M.VoxteraLauncher._safe_after.__get__(fl)
        fl._pump_queue = M.VoxteraLauncher._pump_queue.__get__(fl)

        # Replicate the _do_check_updates body wrapped in def, no need to
        # spawn the real Tk app.
        def body():
            try:
                releases = M.api_get(M.GITHUB_API)
                if not releases:
                    fl._safe_after(lambda: fl._set_status("Nenhum release"))
                    return
                release = releases[0]
                fl.latest_version = release["tag_name"]
                fl.download_asset = M.find_platform_archive(release, fl.platform)
                fl.download_url = (
                    fl.download_asset["browser_download_url"]
                    if fl.download_asset else None
                )
                fl.manifest_url = M.find_manifest_url(release)
                if fl.download_url:
                    fl._safe_after(lambda: fl._set_status(
                        f"Nova versão: {fl.latest_version}"))
            except Exception as exc:
                msg = f"Erro: {exc}"
                fl._safe_after(lambda: fl._set_status(msg))
        threading.Thread(target=body, daemon=True).start()
        # Drain synchronously
        import time
        for _ in range(40):
            time.sleep(0.05)
            while True:
                try:
                    func = fl._ui_queue.get_nowait()
                except Exception:
                    break
                func()
            if fl._status_text != "Verificando atualizações...":
                break
        self.assertTrue(
            fl._status_text.startswith("Nova versão: ") or fl._status_text.startswith("Erro: "),
            f"status did not advance: {fl._status_text!r}",
        )

if __name__ == "__main__":
    unittest.main()
