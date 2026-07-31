"""Regression tests for platform-specific release archive validation."""

from __future__ import annotations

from pathlib import Path
import tempfile
import unittest
import zipfile

from release_asset_gate import required_files_for_archive, validate_archive


class ReleaseAssetGateTests(unittest.TestCase):
    def make_archive(self, name: str, entries: tuple[str, ...]) -> Path:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / name
        with zipfile.ZipFile(archive, "w") as bundle:
            for entry in entries:
                bundle.writestr(entry, "fixture")
        return archive

    def test_windows_archive_requires_the_windows_executable_and_assets(self) -> None:
        required = required_files_for_archive(Path("Voxtera-windows-x64-v0.4.0.zip"))

        self.assertIn("Voxtera.exe", required)
        self.assertIn("assets/common/canary.canary", required)
        self.assertNotIn("Voxtera.app/Contents/MacOS/Voxtera", required)

    def test_macos_archive_requires_the_app_launcher_binary_and_resources(self) -> None:
        required = required_files_for_archive(Path("Voxtera-macos-universal-v0.4.0.zip"))

        self.assertIn("Voxtera.app/Contents/MacOS/Voxtera", required)
        self.assertIn("Voxtera.app/Contents/MacOS/Voxtera-bin", required)
        self.assertIn("Voxtera.app/Contents/Info.plist", required)
        self.assertIn("Voxtera.app/Contents/Resources/assets/common/canary.canary", required)
        self.assertNotIn("Voxtera.exe", required)

    def test_macos_archive_is_rejected_when_resources_are_missing(self) -> None:
        archive = self.make_archive(
            "Voxtera-macos-universal-v0.4.0.zip",
            (
                "Voxtera.app/Contents/MacOS/Voxtera",
                "Voxtera.app/Contents/MacOS/Voxtera-bin",
                "Voxtera.app/Contents/Info.plist",
                "Voxtera.app/Contents/Resources/assets/voxygen/logo.ico",
            ),
        )

        self.assertEqual(
            validate_archive(archive),
            ["Voxtera.app/Contents/Resources/assets/common/canary.canary"],
        )

    def test_macos_archive_with_launcher_binary_and_assets_is_accepted(self) -> None:
        archive = self.make_archive(
            "Voxtera-macos-universal-v0.4.0.zip",
            (
                "Voxtera.app/Contents/MacOS/Voxtera",
                "Voxtera.app/Contents/MacOS/Voxtera-bin",
                "Voxtera.app/Contents/Info.plist",
                "Voxtera.app/Contents/Resources/assets/common/canary.canary",
                "Voxtera.app/Contents/Resources/assets/voxygen/logo.ico",
            ),
        )

        self.assertEqual(validate_archive(archive), [])

    def test_archive_with_a_git_lfs_pointer_is_rejected(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / "Voxtera-windows-x64-v0.4.0.zip"
        required = required_files_for_archive(archive)

        with zipfile.ZipFile(archive, "w") as bundle:
            for entry in required:
                content = (
                    "version https://git-lfs.github.com/spec/v1\n"
                    "oid sha256:deadbeef\n"
                    "size 123\n"
                    if entry == "assets/common/canary.canary"
                    else "fixture"
                )
                bundle.writestr(entry, content)

        self.assertEqual(
            validate_archive(archive),
            ["assets/common/canary.canary is a Git LFS pointer"],
        )


if __name__ == "__main__":
    unittest.main()
