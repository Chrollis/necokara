#!/usr/bin/env python3
"""scripts/build-nsis.py

Build the Necokara NSIS installer in one step:

  1. Generate app icons with `tauri icon`.
  2. Generate NSIS header/sidebar BMPs from the source PNGs (Pillow).
  3. Run `tauri build` (release + NSIS).
  4. Rename the produced installer to lowercase `necokara_*`.

Replaces the previous build-nsis.ps1 + nsis-images.ps1 split.

Requires a system Python with Pillow installed (build-time only).

Usage:
  python scripts/build-nsis.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
ICON_DIR = ROOT / "icons"
TAURI_ICON_DIR = ROOT / "src-tauri" / "icons"
NSIS_DIR = ROOT / "src-tauri" / "nsis"
BUNDLE_NSIS_DIR = ROOT / "src-tauri" / "target" / "release" / "bundle" / "nsis"

# NSIS recommended image sizes.
HEADER_SRC = "neco-header-2695.png"
HEADER_SIZE = (150, 57)
SIDEBAR_SRC = "neco-sidebar-1959.png"
SIDEBAR_SIZE = (164, 314)


def log(msg: str) -> None:
    print(msg, flush=True)


def run(cmd: list[str], cwd: Path | None = None) -> None:
    # On Windows, `npx`/`npm` resolve to .cmd shims that CreateProcess cannot
    # launch directly without a shell; use the explicit npx.cmd name.
    import os

    if os.name == "nt":
        cmd = ["npx.cmd" if c == "npx" else c for c in cmd]
    log(f"  > {' '.join(cmd)}")
    subprocess.run(cmd, cwd=cwd, check=True)


def generate_app_icons() -> None:
    log("\n[1/4] tauri icon")
    src = ICON_DIR / "neco-icon-1024.png"
    if not src.exists():
        raise FileNotFoundError(f"source icon not found: {src}")
    run(["npx", "tauri", "icon", str(src), "--output", str(TAURI_ICON_DIR)], ROOT)


def resize_to_bmp(src: Path, dst: Path, width: int, height: int) -> None:
    from PIL import Image

    img = Image.open(src).convert("RGBA")
    # Composite onto a white background, then convert to RGB (BMP 24bpp).
    bg = Image.new("RGB", img.size, (255, 255, 255))
    bg.paste(img, mask=img.split()[3])
    bg = bg.convert("RGB").resize((width, height), Image.LANCZOS)
    bg.save(dst, format="BMP")
    log(f"  generated {dst} ({width}x{height})")


def generate_nsis_images() -> None:
    log("\n[2/4] NSIS installer images")
    try:
        from PIL import Image  # noqa: F401
    except ImportError:
        log("error: Pillow is required (build-time). Install it first:")
        log("  python -m pip install Pillow")
        raise

    NSIS_DIR.mkdir(parents=True, exist_ok=True)
    resize_to_bmp(
        ICON_DIR / HEADER_SRC, NSIS_DIR / "installer-header.bmp",
        *HEADER_SIZE,
    )
    resize_to_bmp(
        ICON_DIR / SIDEBAR_SRC, NSIS_DIR / "installer-sidebar.bmp",
        *SIDEBAR_SIZE,
    )


def tauri_build() -> None:
    log("\n[3/4] tauri build (release + NSIS)")
    run(["npx", "tauri", "build"], ROOT)


def rename_installer() -> None:
    log("\n[4/4] rename installer to lowercase")
    if not BUNDLE_NSIS_DIR.exists():
        raise FileNotFoundError(f"bundle dir not found: {BUNDLE_NSIS_DIR}")

    renamed = False
    for exe in sorted(BUNDLE_NSIS_DIR.glob("Necokara_*.exe")):
        new_name = exe.name.replace("Necokara", "necokara", 1)
        new_path = exe.with_name(new_name)
        if exe.name == new_name:
            log(f"  already lowercase: {exe.name}")
        else:
            exe.rename(new_path)
            log(f"  renamed to: {new_name}")
        renamed = True

    if not renamed:
        lower = sorted(BUNDLE_NSIS_DIR.glob("necokara_*.exe"))
        if lower:
            log(f"  already exists: {lower[0].name}")
        else:
            log("  no installer found to rename")


def main() -> None:
    log("=== Necokara NSIS build ===")
    generate_app_icons()
    generate_nsis_images()
    tauri_build()
    rename_installer()
    log("")
    log("=== build-nsis complete ===")


if __name__ == "__main__":
    sys.exit(main())
