#!/usr/bin/env python
"""Reject release archives that cannot boot because required game assets are absent."""

from __future__ import annotations

from pathlib import Path
import sys
import zipfile

REQUIRED_FILES = (
    "Voxtera.exe",
    "assets/common/canary.canary",
    "assets/voxygen/logo.ico",
)


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"Usage: {Path(argv[0]).name} <release.zip>")
        return 2

    archive = Path(argv[1])
    if not archive.is_file():
        print(f"FAIL: release archive not found: {archive}")
        return 1

    try:
        with zipfile.ZipFile(archive) as bundle:
            entries = set(bundle.namelist())
    except zipfile.BadZipFile:
        print(f"FAIL: invalid ZIP archive: {archive}")
        return 1

    missing = [path for path in REQUIRED_FILES if path not in entries]
    if missing:
        print(
            "FAIL: release archive is missing required runtime files: "
            + ", ".join(missing)
        )
        return 1

    print(f"PASS: {archive.name} contains Voxtera.exe and required assets.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
