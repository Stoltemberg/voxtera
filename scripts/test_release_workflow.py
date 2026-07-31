"""Structural release-workflow contract tests for cross-platform packages."""

from __future__ import annotations

from pathlib import Path
import unittest


WORKFLOW = Path(__file__).parents[1] / ".github" / "workflows" / "release.yml"


class ReleaseWorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_builds_native_macos_binaries_for_both_cpu_architectures(self) -> None:
        self.assertIn("x86_64-apple-darwin", self.workflow)
        self.assertIn("aarch64-apple-darwin", self.workflow)
        self.assertIn("macos-14", self.workflow)
        self.assertIn("macos-15-intel", self.workflow)
        self.assertNotIn("macos-13", self.workflow)

    def test_rejects_macos_binaries_linked_to_the_nix_store(self) -> None:
        self.assertIn("otool -L", self.workflow)
        self.assertIn("/nix/store", self.workflow)

    def test_packages_a_universal_app_with_explicit_assets_path(self) -> None:
        self.assertIn("lipo -create", self.workflow)
        self.assertIn("Contents/MacOS/Voxtera-bin", self.workflow)
        self.assertIn("Contents/Resources/assets", self.workflow)
        self.assertIn("VELOREN_ASSETS", self.workflow)
        self.assertIn("Voxtera-macos-universal", self.workflow)

    def test_manifest_contains_separate_hashes_for_windows_and_macos(self) -> None:
        self.assertIn('"windows-x64"', self.workflow)
        self.assertIn('"macos-universal"', self.workflow)
        self.assertIn("windows_sha256", self.workflow)
        self.assertIn("macos_sha256", self.workflow)

    def test_builds_and_attaches_native_launchers_for_both_platforms(self) -> None:
        self.assertIn("build-launcher-macos", self.workflow)
        self.assertIn("VOXTERA_PYINSTALLER_TARGET_ARCH=universal2", self.workflow)
        self.assertIn("VoxteraLauncher.exe", self.workflow)
        self.assertIn("VoxteraLauncher.app.zip", self.workflow)

if __name__ == "__main__":
    unittest.main()
