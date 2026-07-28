#!/usr/bin/env python
"""Regression gate for Conrod widget-ID type collisions in the Group tab."""

from pathlib import Path
import sys

PANEL = Path(__file__).resolve().parents[1] / "voxygen" / "src" / "hud" / "friends_panel.rs"


def main() -> int:
    source = PANEL.read_text(encoding="utf-8")
    checks = {
        "invitee header has one widget owner": source.count(
            ".set(state.ids.group_invitee_header, ui);"
        ) == 1,
        "member kick ID is not reused as a background": ".set(state.ids.group_member_kick[0], ui);"
        not in source
        and source.count(".set(state.ids.group_member_kick[name_idx], ui)") == 1,
        "member promote ID is not reused as a status dot": ".set(state.ids.group_member_promote[0], ui);"
        not in source
        and source.count(".set(state.ids.group_member_promote[name_idx], ui)") == 1,
    }
    failed = [name for name, passed in checks.items() if not passed]
    if failed:
        print("FAIL: Group tab reuses Conrod IDs across widget types:")
        for name in failed:
            print(f"- {name}")
        return 1
    print("PASS: Group tab widget IDs have one widget type each.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
