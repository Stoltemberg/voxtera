#!/usr/bin/env python
"""Small regression gate for known Voxtera shader compile failures."""

from pathlib import Path
import re
import sys

REPO_ROOT = Path(__file__).resolve().parents[1]
SKYBOX_FRAGMENT = REPO_ROOT / "assets" / "voxygen" / "shaders" / "skybox-frag.glsl"


def main() -> int:
    source = SKYBOX_FRAGMENT.read_text(encoding="utf-8")

    # The WGPU/Naga backend rejects a skybox shader that uses this local
    # attenuation factor without declaring it. This exact failure prevents
    # the client from creating a window.
    uses_cam_attenuation = bool(re.search(r"\bcam_attenuation\b", source))
    declares_cam_attenuation = bool(
        re.search(r"\b(?:vec[234]|float)\s+cam_attenuation\s*=", source)
    )

    if uses_cam_attenuation and not declares_cam_attenuation:
        print(
            "FAIL: skybox-frag.glsl uses cam_attenuation without declaring it; "
            "WGPU/Naga will reject the shader."
        )
        return 1

    print("PASS: skybox-frag.glsl has no undeclared cam_attenuation reference.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
