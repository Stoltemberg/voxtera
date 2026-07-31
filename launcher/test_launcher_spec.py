"""Contract test for building a native macOS launcher bundle."""

from __future__ import annotations

from pathlib import Path
import unittest


SPEC = Path(__file__).with_name("VoxteraLauncher.spec")
LAUNCHER = Path(__file__).with_name("voxtera_launcher.py")


class LauncherSpecTests(unittest.TestCase):
    def test_wraps_the_launcher_in_an_app_bundle_on_macos(self) -> None:
        spec = SPEC.read_text(encoding="utf-8")

        self.assertIn("from PyInstaller.building.osx import BUNDLE", spec)
        self.assertIn('sys.platform == "darwin"', spec)
        self.assertIn("VoxteraLauncher.app", spec)
        self.assertIn("bundle_identifier", spec)
        self.assertIn('exclude_binaries=sys.platform == "darwin"', spec)
        self.assertIn("COLLECT(", spec)
        self.assertIn('os.environ.get("VOXTERA_PYINSTALLER_TARGET_ARCH")', spec)

    def test_declares_a_visible_launcher_version_in_the_macos_bundle(self) -> None:
        spec = SPEC.read_text(encoding="utf-8")

        self.assertIn("VOXTERA_LAUNCHER_VERSION", spec)
        self.assertIn("CFBundleShortVersionString", spec)
        self.assertIn("CFBundleVersion", spec)

    def test_uses_tkinter_for_the_logo_without_native_pillow_extensions(self) -> None:
        launcher = LAUNCHER.read_text(encoding="utf-8")

        self.assertIn("tk.PhotoImage", launcher)
        self.assertNotIn("from PIL", launcher)


if __name__ == "__main__":
    unittest.main()
