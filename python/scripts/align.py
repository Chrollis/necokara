#!/usr/bin/env python3
"""scripts/align.py

Force-align a lyrics text onto an audio file with stable-ts, and emit
per-character timestamps as JSON on stdout.

Contract (called by the Rust wrapper):
  argv: --vocals PATH (--lyrics TEXT | --lyrics-file PATH) --lang CODE
        --model-dir DIR [--ffmpeg PATH]
  stdout: one JSON object
      {"ok": true, "tokens": [{"index": 0, "char": "x", "time": 1.23}, ...]}
      {"ok": false, "error": "message"}
  `index` is the character offset into the ORIGINAL lyrics text; `time` is in
  seconds. Errors print nothing to stdout except the final JSON; diagnostics
  go to stderr.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

import numpy as np

FFMPEG_SAMPLE_RATE = 16000


def decode_audio(audio_path: str, ffmpeg_path: str) -> np.ndarray:
    """Decode audio to 16 kHz mono float32 PCM via ffmpeg."""
    cmd = [
        ffmpeg_path,
        "-nostdin",
        "-loglevel", "error",
        "-i", audio_path,
        "-f", "f32le",
        "-ac", "1",
        "-ar", str(FFMPEG_SAMPLE_RATE),
        "-",
    ]
    proc = subprocess.run(cmd, capture_output=True, check=True)
    samples = np.frombuffer(proc.stdout, dtype=np.float32).copy()
    if samples.size == 0:
        raise RuntimeError(f"ffmpeg decoded no audio from {audio_path}")
    return samples


def standardize_text(text: str) -> str:
    """Collapse whitespace and add a leading space (matches stable-ts)."""
    text = re.sub(r"\s", " ", text)
    if not text.startswith(" "):
        text = " " + text
    return text


def build_tokens(result, lyrics: str) -> list[dict]:
    """Extract char-level times from the stable-ts result.

    stable-ts aligns the standardized text (whitespace collapsed, leading
    space). Each word maps back to a run of the ORIGINAL lyrics; characters
    are emitted in order (whitespace-collapse differences are bridged by
    indexing the original text), so the caller can consume the tokens
    sequentially against its own char stream.
    """
    tokens = []
    std_offset = 0
    for seg in result.segments:
        for w in seg.words or []:
            n = len(w.word)
            if n == 0:
                continue
            for j in range(n):
                std_idx = std_offset + j
                if std_idx == 0:  # the artificial leading space
                    continue
                frac = j / n if n > 1 else 0.0
                t = round(w.start + (w.end - w.start) * frac, 3)
                char_idx = std_idx - 1
                if char_idx < len(lyrics):
                    tokens.append({"char": lyrics[char_idx], "time": t})
            std_offset += n
    return tokens


def main() -> int:
    parser = argparse.ArgumentParser(description="lyrics alignment")
    parser.add_argument("--vocals", required=True)
    parser.add_argument("--lang", required=True)
    parser.add_argument("--model-dir", required=True)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    lyrics_group = parser.add_mutually_exclusive_group()
    lyrics_group.add_argument("--lyrics")
    lyrics_group.add_argument("--lyrics-file")
    args = parser.parse_args()

    if args.lyrics_file is not None:
        try:
            lyrics = Path(args.lyrics_file).read_text(encoding="utf-8")
        except Exception as exc:  # noqa: BLE001
            print(json.dumps({"ok": False, "error": str(exc)}, ensure_ascii=False))
            return 1
    else:
        lyrics = args.lyrics or ""

    if not lyrics:
        print(json.dumps({"ok": False, "error": "--lyrics or --lyrics-file required"}, ensure_ascii=False))
        return 1

    try:
        import stable_whisper  # type: ignore

        sys.stderr.write(f"[align] decoding {args.vocals} ...\n")
        audio = decode_audio(args.vocals, args.ffmpeg)
        sys.stderr.write(f"[align] {audio.size / FFMPEG_SAMPLE_RATE:.1f}s audio\n")

        sys.stderr.write(f"[align] loading model from {args.model_dir} ...\n")
        model = stable_whisper.load_faster_whisper(
            args.model_dir, device="auto", compute_type="auto"
        )

        sys.stderr.write(f"[align] aligning {len(lyrics)} chars ...\n")
        result = model.align(audio, lyrics, language=args.lang, verbose=None)
        if result is None:
            raise RuntimeError("stable-ts align returned no result")

        tokens = build_tokens(result, lyrics)
        sys.stderr.write(f"[align] done: {len(tokens)} tokens\n")
        print(json.dumps({"ok": True, "tokens": tokens}, ensure_ascii=False))
        return 0
    except Exception as exc:  # noqa: BLE001
        import traceback

        traceback.print_exc(file=sys.stderr)
        print(json.dumps({"ok": False, "error": f"{type(exc).__name__}: {exc}"}, ensure_ascii=False))
        return 1


if __name__ == "__main__":
    sys.exit(main())
