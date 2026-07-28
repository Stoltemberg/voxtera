"""Voxtera localization gate.

Two checks:

1. Extract every ``Content::localized("key")`` and ``Content::localized_with_args("key", ...)``
   reference from Rust source, and verify each key exists in at least the EN and PT-BR
   localization files.

2. Detect hardcoded English-looking literals that flow into the chat via
   ``into_plain_msg(format!(...))`` or ``into_plain_msg(String::from(...))`` in the
   client and server entry points. The list of approved exceptions lives in this
   file (whitelist). Anything else fails the gate.

Run via ``scripts/validate_chat_localization.bat`` before packaging a release.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
I18N_ROOT = REPO / "assets" / "voxygen" / "i18n"
EN_DIR = I18N_ROOT / "en"
PTBR_DIR = I18N_ROOT / "pt-BR"
SCAN_DIRS = [
    REPO / "client" / "src",
    REPO / "server" / "src",
    REPO / "voxygen" / "src",
]

LOCALIZED_RE = re.compile(
    r'Content::localized(?:_with_args)?\(\s*"([a-zA-Z][a-zA-Z0-9_.-]*)"',
)
FTL_KEY_RE = re.compile(r'^([a-zA-Z][a-zA-Z0-9_.-]*)\s*=', re.MULTILINE)

# Whitelist of literal EN strings that legitimately flow into chat. Add new
# entries only when the string cannot be a Fluent key.
ALLOWED_LITERALS: set[str] = set()


def collect_ftl_keys(directory: Path) -> set[str]:
    keys: set[str] = set()
    if not directory.exists():
        return keys
    for f in sorted(directory.rglob("*.ftl")):
        text = f.read_text(encoding="utf-8-sig")
        for match in FTL_KEY_RE.finditer(text):
            keys.add(match.group(1))
    return keys


def collect_used_keys() -> set[str]:
    used: set[str] = set()
    for scan_dir in SCAN_DIRS:
        if not scan_dir.exists():
            continue
        for source in scan_dir.rglob("*.rs"):
            text = source.read_text(encoding="utf-8-sig")
            used.update(match.group(1) for match in LOCALIZED_RE.finditer(text))
    return used


def collect_chat_literals() -> list[tuple[str, int, str]]:
    suspects: list[tuple[str, int, str]] = []
    rg = subprocess.run(
        ["rg", "-n", "--type=rust", "-S",
         r'into_plain_msg\((?:format!|String::from)\(',
         *SCAN_DIRS],
        text=True,
        capture_output=True,
        check=False,
    )
    for line in rg.stdout.splitlines():
        path, lineno, payload = line.split(":", 2)
        text = payload.strip()
        for needle in ALLOWED_LITERALS:
            if needle in text:
                break
        else:
            suspects.append((path, int(lineno), text))
    return suspects


def main() -> int:
    failures: list[str] = []

    used = collect_used_keys()
    en_keys = collect_ftl_keys(EN_DIR)
    pt_keys = collect_ftl_keys(PTBR_DIR)

    missing_en = sorted(k for k in used if k not in en_keys)
    missing_pt = sorted(k for k in used if k not in pt_keys)

    if missing_en:
        failures.append(
            "Missing keys in EN localization:\n  - " + "\n  - ".join(missing_en)
        )
    if missing_pt:
        failures.append(
            "Missing keys in PT-BR localization:\n  - " + "\n  - ".join(missing_pt)
        )

    literals = collect_chat_literals()
    if literals:
        formatted = "\n".join(f"  {p}:{n}: {t}" for p, n, t in literals)
        failures.append(
            "Hardcoded string literals flowing into chat via into_plain_msg(...):\n"
            + formatted
        )

    report = {
        "used_keys": sorted(used),
        "missing_en": missing_en,
        "missing_pt": missing_pt,
        "literal_suspects": [
            {"path": p, "line": n, "text": t} for p, n, t in literals
        ],
        "pass": not failures,
    }
    print(json.dumps(report, indent=2, ensure_ascii=False))

    if failures:
        for f in failures:
            print("FAIL: " + f, file=sys.stderr)
        return 1
    print("PASS: chat localization gate clean", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
