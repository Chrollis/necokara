#!/usr/bin/env python3
"""scripts/create-env.py

Download the Windows embeddable Python + ffmpeg and place them under
binaries/. Produces the *minimal* runtime (no third-party packages, no AI
models) plus a copy under binaries/mini/ used as the packaging baseline.

Output:
  binaries/            full layout (python embed + pip + ffmpeg) — dev target
  binaries/mini/       copy of the same minimal set — packaged into the
                       installer (tauri resources -> resources/binaries)

The heavy parts (torch, AI deps, models) are installed later by setup-env.py,
so the packaged mini only carries python embed + pip + ffmpeg + licenses.

Usage:
  python scripts/create-env.py            # official download sources
"""

from __future__ import annotations

import argparse
import shutil
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
BINARIES_DIR = ROOT / "binaries"
MINI_DIR = BINARIES_DIR / "mini"
CACHE_DIR = ROOT / "download"

PYTHON_VERSION = "3.12.10"
PYTHON_EMBED_URL = (
    f"https://www.python.org/ftp/python/{PYTHON_VERSION}/"
    f"python-{PYTHON_VERSION}-embed-amd64.zip"
)
PYTHON_ZIP_CACHE = CACHE_DIR / f"python-{PYTHON_VERSION}-embed-amd64.zip"
GET_PIP_URL = "https://bootstrap.pypa.io/get-pip.py"
GET_PIP_CACHE = CACHE_DIR / "get-pip.py"
PIP_MIRROR = "https://mirrors.aliyun.com/pypi/simple/"
FFMPEG_URL = (
    "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/"
    "ffmpeg-master-latest-win64-gpl.zip"
)
FFMPEG_ZIP_CACHE = CACHE_DIR / "ffmpeg-gpl.zip"
FFMPEG_FOLDER_NAME = "ffmpeg-master-latest-win64-gpl"


def log(msg: str) -> None:
    print(msg, flush=True)


def download_file(url: str, dest: Path, retries: int = 3) -> None:
    """Download url to dest (skip when a non-empty file already exists)."""
    import urllib.request

    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 0:
        log(f"  exists, skipping {dest.name}")
        return
    for attempt in range(1, retries + 1):
        try:
            log(f"  downloading {dest.name} from {url}")
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req) as resp, open(dest, "wb") as f:
                shutil.copyfileobj(resp, f)
            log(f"    -> {dest}")
            return
        except Exception as err:  # noqa: BLE001
            log(f"  attempt {attempt}/{retries} failed: {err}")
            if attempt < retries:
                log("  retrying...")
            else:
                raise
    raise RuntimeError(f"failed to download {url}")


def unzip(zip_path: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(zip_path) as zf:
        zf.extractall(dest)


def ensure_python() -> None:
    """Download + extract the embeddable Python into binaries/ and add pip."""
    log("\n[1/3] python embed runtime")

    pip_marker = BINARIES_DIR / "Lib" / "site-packages" / "pip"
    if (BINARIES_DIR / "python.exe").exists() and pip_marker.exists():
        log("  python embed + pip already present in binaries/")
        return

    # Partial/failed state -> rebuild from scratch.
    if BINARIES_DIR.exists():
        shutil.rmtree(BINARIES_DIR)
    BINARIES_DIR.mkdir(parents=True, exist_ok=True)

    download_file(PYTHON_EMBED_URL, PYTHON_ZIP_CACHE)
    unzip(PYTHON_ZIP_CACHE, BINARIES_DIR)

    # Configure the ._pth: enable site-packages WITHOUT "import site".
    # Adding "import site" would activate the full site machinery and pull in
    # the user site-packages (%APPDATA%\Python\...), breaking runtime
    # isolation (pip would see unrelated packages such as manim). The explicit
    # paths below are enough for pip-installed packages under Lib\site-packages.
    pth_name = next(
        (f.name for f in BINARIES_DIR.iterdir() if f.name.endswith("._pth")),
        "python312._pth",
    )
    pth_path = BINARIES_DIR / pth_name
    pth_path.write_text(
        "python312.zip\n.\nLib\\site-packages\n",
        encoding="utf-8",
    )

    # Install pip into the embeddable python.
    download_file(GET_PIP_URL, GET_PIP_CACHE)
    py = BINARIES_DIR / "python.exe"
    run_cmd(
        [str(py), str(GET_PIP_CACHE), "--no-warn-script-location"],
        env={"TMP": str(CACHE_DIR / "tmp"), "TEMP": str(CACHE_DIR / "tmp")},
    )
    log("  python embed + pip ready")


def run_cmd(cmd: list[str], env: dict[str, str] | None = None) -> None:
    import os
    import subprocess

    log(f"  > {' '.join(cmd)}")
    full_env = dict(os.environ)
    if env:
        full_env.update(env)
    subprocess.run(cmd, check=True, env=full_env)


def ensure_ffmpeg() -> None:
    """Download the GPL ffmpeg build and copy exe + license into binaries/."""
    log("\n[2/3] ffmpeg (GPL)")
    if (BINARIES_DIR / "ffmpeg.exe").exists():
        log("  ffmpeg already present in binaries/")
        return

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    download_file(FFMPEG_URL, FFMPEG_ZIP_CACHE)

    # Extract under the cache dir (never pollute the repo root), then locate
    # bin/ffmpeg.exe wherever the zip puts it (the build zip carries a top
    # folder of its own; a stale nested copy would otherwise double it).
    extract_root = CACHE_DIR / "ffmpeg_extract"
    if extract_root.exists():
        shutil.rmtree(extract_root)
    unzip(FFMPEG_ZIP_CACHE, extract_root)

    # Find the *first* bin/ffmpeg.exe under the extraction root.
    matches = sorted(extract_root.rglob("bin/ffmpeg.exe"))
    if not matches:
        raise RuntimeError("ffmpeg.exe not found after extraction")
    bin_src = matches[0].parent

    shutil.copy2(bin_src / "ffmpeg.exe", BINARIES_DIR / "ffmpeg.exe")
    shutil.copy2(bin_src / "ffprobe.exe", BINARIES_DIR / "ffprobe.exe")

    # LICENSE.txt sits next to the top-level build folder.
    license_src = extract_root / FFMPEG_FOLDER_NAME / "LICENSE.txt"
    if not license_src.exists():
        license_src = extract_root / "LICENSE.txt"
    if license_src.exists():
        shutil.copy2(license_src, BINARIES_DIR / "LICENSE.ffmpeg.txt")

    shutil.rmtree(extract_root, ignore_errors=True)
    log("  ffmpeg ready in binaries/")


def make_mini() -> None:
    """Copy the minimal runtime (python + ffmpeg, no third-party pkgs) to
    binaries/mini/ for packaging."""
    log("\n[3/3] mini copy (packaging baseline)")
    if MINI_DIR.exists():
        shutil.rmtree(MINI_DIR)
    MINI_DIR.mkdir(parents=True, exist_ok=True)

    # Copy the whole binaries/ tree (at create time it only contains the
    # embed runtime + pip + ffmpeg, which is exactly the mini baseline).
    for item in BINARIES_DIR.iterdir():
        if item.name == "mini":
            continue
        if item.is_dir():
            shutil.copytree(item, MINI_DIR / item.name)
        else:
            shutil.copy2(item, MINI_DIR / item.name)
    log(f"  mini baseline at {MINI_DIR}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Create the minimal Necokara runtime")
    parser.add_argument("--mirror", "-m", action="store_true",
                        help="use China mirrors for downloads")
    args = parser.parse_args()

    global PYTHON_EMBED_URL
    if args.mirror:
        PYTHON_EMBED_URL = (
            "https://mirror.nju.edu.cn/python/3.12.10/"
            "python-3.12.10-embed-amd64.zip"
        )
        # pip index mirror is applied in setup-env; get-pip itself is small.

    log("=== Necokara create-env ===")
    log(f"binaries: {BINARIES_DIR}")
    log("")

    ensure_python()
    ensure_ffmpeg()
    make_mini()

    log("")
    log("=== create-env complete ===")
    log("Next: python scripts/setup-env.py [--mirror] [--cuda]")


if __name__ == "__main__":
    sys.exit(main())
