"""Contract test for building a native macOS launcher bundle."""

from __future__ import annotations

from pathlib import Path
import unittest


SPEC = Path(__file__).with_name("VoxteraLauncher.spec")
LAUNCHER = Path(__file__).with_name("voxtera_launcher.py")
REQUIREMENTS = Path(__file__).with_name("requirements.txt")
BUILD_REQUIREMENTS = Path(__file__).with_name("requirements-build.txt")


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

    def test_collects_certifi_ca_bundle_for_the_frozen_launcher(self) -> None:
        spec = SPEC.read_text(encoding="utf-8")

        self.assertIn("collect_data_files", spec)
        self.assertIn('collect_data_files("certifi"', spec)
        self.assertIn("CERTIFI_DATAS", spec)

    def test_declares_certifi_as_a_launcher_runtime_dependency(self) -> None:
        requirements = REQUIREMENTS.read_text(encoding="utf-8")

        self.assertIn("certifi==2026.7.22", requirements)

    def test_declares_a_reproducible_pyinstaller_build_environment(self) -> None:
        requirements = BUILD_REQUIREMENTS.read_text(encoding="utf-8")

        self.assertIn("-r requirements.txt", requirements)
        self.assertIn("pyinstaller==6.21.0", requirements)

    def test_uses_tkinter_for_the_logo_without_native_pillow_extensions(self) -> None:
        launcher = LAUNCHER.read_text(encoding="utf-8")

        self.assertIn("tk.PhotoImage", launcher)
        self.assertNotIn("from PIL", launcher)


if __name__ == "__main__":
    unittest.main()
