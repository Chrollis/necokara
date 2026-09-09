#!/usr/bin/env python3
"""scripts/separate.py

Separate an audio file into vocals and accompaniment (karaoke mode) with
demucs-infer.

Contract (called by the Rust wrapper):
  argv: --src PATH --vocals PATH --accompaniment PATH --model-dir DIR
        [--ffmpeg PATH]
  stdout: one JSON object
      {"ok": true}
      {"ok": false, "error": "message"}
  The vocals stem is written as wav to --vocals; all other stems are mixed
  into a single accompaniment wav written to --accompaniment.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


def fail(message: str) -> int:
    print(json.dumps({"ok": False, "error": message}, ensure_ascii=False))
    return 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--src", required=True)
    parser.add_argument("--vocals", required=True)
    parser.add_argument("--accompaniment", required=True)
    parser.add_argument("--model-dir", required=True)
    parser.add_argument("--ffmpeg", help="path to ffmpeg executable (decoding)")
    args = parser.parse_args()

    # Make ffmpeg discoverable for the audio decoders if provided.
    if args.ffmpeg:
        ff_dir = str(Path(args.ffmpeg).resolve().parent)
        os.environ["PATH"] = ff_dir + os.pathsep + os.environ.get("PATH", "")

    # NOTE: progress cannot be tracked from here; the frontend shows fake
    # progress. Keep stdout clean for the final JSON only.
    try:
        import torch  # noqa: F401
        from demucs_infer.api import Separator  # type: ignore
        from demucs_infer.audio import save_audio  # type: ignore
    except Exception as exc:  # noqa: BLE001
        return fail(f"demucs-infer not importable: {exc}")

    try:
        device = "cuda" if torch.cuda.is_available() else "cpu"
        # Model weights live in a local repo dir (setup-env layout:
        # Models/demucs/htdemucs.yaml + <sig>.th).
        sys.stderr.write("[separate] loading demucs-infer ...\n")
        separator = Separator(model="htdemucs", repo=Path(args.model_dir), device=device)

        sys.stderr.write("[separate] separating ...\n")
        origin, stems = separator.separate_audio_file(args.src)
        # `stems` maps stem name -> separated waveform (matching samplerate).
        del origin

        vocals = stems["vocals"]
        acc = None
        for name, wav in stems.items():
            if name == "vocals":
                continue
            acc = wav if acc is None else acc + wav

        out_dir = Path(args.vocals).parent
        out_dir.mkdir(parents=True, exist_ok=True)

        sys.stderr.write(f"[separate] saving vocals -> {args.vocals}\n")
        save_audio(vocals, args.vocals, separator.samplerate)
        sys.stderr.write(f"[separate] saving accompaniment -> {args.accompaniment}\n")
        save_audio(acc, args.accompaniment, separator.samplerate)

        print(json.dumps({"ok": True}, ensure_ascii=False))
        return 0
    except Exception as exc:  # noqa: BLE001
        import traceback

        traceback.print_exc(file=sys.stderr)
        return fail(f"separation failed: {exc}")


if __name__ == "__main__":
    sys.exit(main())
