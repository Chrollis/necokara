#!/usr/bin/env python3
"""scripts/setup-env.py

Install the heavy AI components (torch CPU/CUDA, AI packages, models) into a
minimal runtime that was created by create-env.py.

Works both for development (runtime = ./binaries) and inside the installer
(runtime = <install dir>/resources/binaries, after the NSIS setup page runs
this script with --runtime).

Usage:
  python scripts/setup-env.py                 # CPU torch, official index
  python scripts/setup-env.py --cuda          # CUDA 12.4 torch
  python scripts/setup-env.py --mirror        # China mirrors (pip + HF)
  python scripts/setup-env.py --runtime PATH  # install into PATH (default ./binaries)
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_RUNTIME = ROOT / "binaries"
CACHE_DIR = ROOT / "download"
MODEL_CACHE_DIR = CACHE_DIR / "models"

# AI package dependencies installed by pip (besides torch/torchaudio).
# soundfile is a transitive dep of librosa/demucs-infer (pip pulls it in);
# not listed explicitly. Keep in sync with python/requirements.txt.
AI_REQUIREMENTS = [
    "librosa>=0.11.0",
    "numpy>=2.5.0",
    "janome>=0.5.0",
    "stable-ts>=2.19.1",
    "faster-whisper>=1.2.1",
    "demucs-infer>=4.2.2",
]

PIP_MIRROR = "https://mirrors.aliyun.com/pypi/simple/"
HF_MIRROR = "https://hf-mirror.com"
TORCH_CPU_INDEX = "https://download.pytorch.org/whl/cpu"
TORCH_CUDA_INDEX = "https://download.pytorch.org/whl/cu124"

# Whisper model files (Systran/faster-whisper-base).
WHISPER_REPO = "Systran/faster-whisper-base"
WHISPER_FILES = ["config.json", "model.bin", "tokenizer.json", "vocabulary.txt"]
DEMUCS_WEIGHT_URL = (
    "https://dl.fbaipublicfiles.com/demucs/hybrid_transformer/"
    "955717e8-8726e21a.th"
)
DEMUCS_WEIGHT_NAME = "955717e8-8726e21a.th"


def log(msg: str) -> None:
    print(msg, flush=True)


def run_cmd(cmd: list[str], env: dict[str, str] | None = None) -> None:
    import os

    log(f"  > {' '.join(cmd)}")
    full_env = dict(os.environ)
    if env:
        full_env.update(env)
    subprocess.run(cmd, check=True, env=full_env)


def download_file(url: str, dest: Path, retries: int = 3) -> None:
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


def pip_index_args(mirror: bool) -> list[str]:
    if mirror:
        return ["-i", PIP_MIRROR, "--trusted-host", "mirrors.aliyun.com"]
    return []


def install_dependencies(python: Path, mirror: bool, cuda: bool) -> None:
    log("\n[1/3] installing AI packages with pip...")

    # Give pip a local scratch dir (safer/cleaner than the system temp,
    # especially inside the installer where temp may be restricted).
    pip_tmp = python.parent / ".pip_tmp"
    pip_tmp.mkdir(parents=True, exist_ok=True)
    pip_env = {"TMP": str(pip_tmp), "TEMP": str(pip_tmp)}

    # torch + torchaudio from the pytorch index (cpu/cu124).
    torch_index = TORCH_CUDA_INDEX if cuda else TORCH_CPU_INDEX
    run_cmd(
        [
            str(python), "-m", "pip", "install", "--no-warn-script-location",
            "--index-url", torch_index, "torch", "torchaudio",
        ],
        env=pip_env,
    )

    # Other AI requirements from the (possibly mirrored) general index.
    run_cmd(
        [
            str(python), "-m", "pip", "install", "--no-warn-script-location",
            "--no-build-isolation", *pip_index_args(mirror),
            *AI_REQUIREMENTS,
        ],
        env=pip_env,
    )


def download_models(runtime: Path, mirror: bool) -> None:
    log("\n[2/3] downloading models...")

    cache_whisper = MODEL_CACHE_DIR / "whisper"
    cache_demucs = MODEL_CACHE_DIR / "demucs"
    hf_base = HF_MIRROR if mirror else "https://huggingface.co/"

    for name in WHISPER_FILES:
        url = f"{hf_base}{WHISPER_REPO}/resolve/main/{name}"
        download_file(url, cache_whisper / name)

    download_file(DEMUCS_WEIGHT_URL, cache_demucs / DEMUCS_WEIGHT_NAME)
    cache_demucs.mkdir(parents=True, exist_ok=True)
    (cache_demucs / "htdemucs.yaml").write_text(
        "models: ['955717e8']\n", encoding="utf-8"
    )

    # Copy into the runtime.
    models_dir = runtime / "Models"
    shutil.copytree(cache_whisper, models_dir / "whisper", dirs_exist_ok=True)
    shutil.copytree(cache_demucs, models_dir / "demucs", dirs_exist_ok=True)
    log(f"  models copied to {models_dir}")


def verify(runtime: Path) -> None:
    log("\n[3/3] verifying runtime...")
    python = runtime / "python.exe"
    run_cmd([str(python), "-c", "import torch; print('torch', torch.__version__)"])
    run_cmd([str(python), "-c", "import faster_whisper, stable_whisper, demucs_infer; print('AI deps ok')"])


def main() -> None:
    parser = argparse.ArgumentParser(description="Install Necokara AI components")
    parser.add_argument("--cuda", action="store_true", help="install CUDA 12.4 torch")
    parser.add_argument("--mirror", "-m", action="store_true", help="use China mirrors")
    parser.add_argument("--no-model", "-M", action="store_true", help="skip model downloads")
    parser.add_argument(
        "--runtime",
        type=Path,
        default=DEFAULT_RUNTIME,
        help="runtime directory to install into (default: ./binaries)",
    )
    args = parser.parse_args()

    runtime = args.runtime.resolve()
    python = runtime / "python.exe"
    if not python.exists():
        log(f"error: no python.exe in {runtime} (run create-env.py first)")
        return 1

    log("=== Necokara setup-env ===")
    log(f"runtime : {runtime}")
    log(f"cuda    : {args.cuda}")
    log(f"mirror  : {args.mirror}")
    log(f"models  : {'skipped' if args.no_model else 'enabled'}")
    log("")

    install_dependencies(python, args.mirror, args.cuda)
    if not args.no_model:
        download_models(runtime, args.mirror)
    verify(runtime)

    log("")
    log("=== setup-env complete ===")
    return 0


if __name__ == "__main__":
    sys.exit(main())
