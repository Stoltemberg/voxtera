#!/usr/bin/env python
"""Reject release archives that cannot boot because required game assets are absent."""

from __future__ import annotations

from pathlib import Path
import sys
import zipfile

WINDOWS_REQUIRED_FILES = (
    "Voxtera.exe",
    "assets/common/canary.canary",
    "assets/voxygen/logo.ico",
)

MACOS_REQUIRED_FILES = (
    "Voxtera.app/Contents/Info.plist",
    "Voxtera.app/Contents/MacOS/Voxtera",
    "Voxtera.app/Contents/MacOS/Voxtera-bin",
    "Voxtera.app/Contents/Resources/assets/common/canary.canary",
    "Voxtera.app/Contents/Resources/assets/voxygen/logo.ico",
)
GIT_LFS_POINTER_PREFIX = b"version https://git-lfs.github.com/spec/v1\n"


def required_files_for_archive(archive: Path) -> tuple[str, ...]:
    name = archive.name
    if "windows-x64" in name:
        return WINDOWS_REQUIRED_FILES
    if "macos-universal" in name:
        return MACOS_REQUIRED_FILES
    raise ValueError(f"Unsupported release archive name: {name}")


def validate_archive(archive: Path) -> list[str]:
    required_files = required_files_for_archive(archive)
    with zipfile.ZipFile(archive) as bundle:
        entries = set(bundle.namelist())
        errors = [path for path in required_files if path not in entries]
        for path in required_files:
            if path in entries and ("/assets/" in path or path.startswith("assets/")):
                if bundle.read(path).startswith(GIT_LFS_POINTER_PREFIX):
                    errors.append(f"{path} is a Git LFS pointer")
    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"Usage: {Path(argv[0]).name} <release.zip>")
        return 2

    archive = Path(argv[1])
    if not archive.is_file():
        print(f"FAIL: release archive not found: {archive}")
        return 1

    try:
        missing = validate_archive(archive)
    except zipfile.BadZipFile:
        print(f"FAIL: invalid ZIP archive: {archive}")
        return 1
    except ValueError as error:
        print(f"FAIL: {error}")
        return 1

    if missing:
        print(
            "FAIL: release archive is missing required runtime files: "
            + ", ".join(missing)
        )
        return 1

    print(f"PASS: {archive.name} contains the platform executable and required assets.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
