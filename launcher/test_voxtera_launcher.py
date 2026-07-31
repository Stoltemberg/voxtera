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


if __name__ == "__main__":
    unittest.main()
