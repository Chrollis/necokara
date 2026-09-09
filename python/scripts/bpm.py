#!/usr/bin/env python3
"""scripts/bpm.py

Detect BPM segments over an audio file with librosa.

Approach (unlike the old Electron window-tempo averaging):
  1. ffmpeg decodes the source to a temp wav; librosa loads it.
  2. An onset-strength envelope is computed once.
  3. A per-frame (time-varying) tempo estimate is computed from it.
  4. beat_track is run with that time-varying tempo as the bpm hint, giving
     the position of every beat (`units='time'`).
  5. Adjacent beats give an instantaneous bpm (60 / inter-beat interval).
     Raw per-beat bpm jitters (a ~500 ms beat at 120 BPM shifts +-30 ms ->
     +-7 BPM), so a small median filter removes single-beat outliers before
     segmenting; only a sustained tempo change crosses `--threshold`
     (absolute bpm difference). A segment starts at its first detected beat,
     so the start is a real beat position - the phase the UI needs to draw a
     beat grid (`beat k = start + k * 60000/bpm`).

Contract (called by the Rust wrapper):
  argv: --input PATH --ffmpeg PATH [--threshold FLOAT] [--window INT]
  stdout: one JSON object
      {"ok": true, "segments": [{"bpm": 120.0, "start_ms": 1234}, ...]}
      {"ok": false, "error": "..."}
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def fail(message: str) -> int:
    print(json.dumps({"ok": False, "error": message}, ensure_ascii=False))
    return 1


def decode_to_wav(input_path: str, ffmpeg: str) -> str:
    """Decode any audio to mono 44.1k wav (what librosa reads best)."""
    tmp = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
    tmp.close()
    cmd = [
        ffmpeg,
        "-nostdin",
        "-loglevel", "error",
        "-y",
        "-i", input_path,
        "-acodec", "pcm_s16le",
        "-ar", "44100",
        "-ac", "1",
        tmp.name,
    ]
    subprocess.run(cmd, check=True)
    return tmp.name


def median(values: list[float]) -> float:
    s = sorted(values)
    return s[len(s) // 2]


def median_filter(values: list[float], width: int) -> list[float]:
    """Median filter over `values` with a centred window of `width`
    (rounded to an odd number)."""
    half = max(1, (width - 1) // 2)
    out: list[float] = []
    for i in range(len(values)):
        lo = max(0, i - half)
        hi = min(len(values), i + half + 1)
        out.append(median(values[lo:hi]))
    return out


def build_segments(
    beat_times: list[float],
    threshold: float,
    window: int = 5,
    merge_ratio: float = 1.0,
) -> list[dict]:
    """Group consecutive beats whose (median-filtered) instantaneous bpm
    stays within `threshold` (absolute bpm) of the current segment's bpm.

    Per-beat bpm jitters (a ~500 ms beat at 120 BPM shifted +-30 ms is
    +-7 BPM), so a median filter of width `window` removes single-beat
    outliers first; only a sustained tempo change crosses the threshold. A
    segment's start is its first detected beat (a real beat position).

    After the first cut, adjacent segments whose bpm differs by less than
    `threshold * merge_ratio` are merged (ratio 1.0 = same as the cut
    threshold; larger ratios fold near-misses back together).
    """
    if len(beat_times) < 2:
        return []

    # Instantaneous bpm per interval (interval i = beat i -> beat i+1).
    inst_bpm: list[float] = []
    for i in range(len(beat_times) - 1):
        interval_s = beat_times[i + 1] - beat_times[i]
        if interval_s > 0:
            inst_bpm.append(60.0 / interval_s)

    if not inst_bpm:
        return []

    filtered = median_filter(inst_bpm, window)

    # First pass: cut where the filtered bpm departs from the running
    # segment median by more than `threshold`.
    segments: list[dict] = []
    seg_start_i = 0
    seg_bpms: list[float] = [filtered[0]]
    bench = filtered[0]

    for i in range(1, len(filtered)):
        if abs(filtered[i] - bench) > threshold:
            segments.append({
                "bpm": median(seg_bpms),
                "start_i": seg_start_i,
            })
            seg_start_i = i
            seg_bpms = [filtered[i]]
            bench = filtered[i]
        else:
            seg_bpms.append(filtered[i])
            bench = median(seg_bpms)

    segments.append({
        "bpm": median(seg_bpms),
        "start_i": seg_start_i,
    })

    # Second pass: merge adjacent segments whose bpm differs by less than
    # `threshold * merge_ratio` (keeps the earlier start; bpm weighted by
    # segment length). Segment length = next start_i - this start_i (last
    # one to len(filtered)).
    merge_limit = threshold * merge_ratio
    n = len(filtered)
    merged: list[dict] = []
    for idx, seg in enumerate(segments):
        end_i = segments[idx + 1]["start_i"] if idx + 1 < len(segments) else n
        seg["len"] = end_i - seg["start_i"]
        if merged and abs(seg["bpm"] - merged[-1]["bpm"]) < merge_limit:
            last = merged[-1]
            w = last["len"] + seg["len"]
            merged[-1] = {
                "bpm": (last["bpm"] * last["len"] + seg["bpm"] * seg["len"]) / w,
                "start_i": last["start_i"],
                "len": w,
            }
        else:
            merged.append(seg)

    return [
        {
            "bpm": round(s["bpm"], 4),
            "start_ms": int(round(beat_times[s["start_i"]] * 1000)),
        }
        for s in merged
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description="librosa BPM segment detection")
    parser.add_argument("--input", required=True)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--threshold", type=float, default=5.0)
    parser.add_argument(
        "--window", type=int, default=5,
        help="median-filter window over per-beat bpm (odd number; larger "
             "smooths more, yielding fewer segments)",
    )
    parser.add_argument(
        "--merge-ratio", type=float, default=1.0,
        help="adjacent segments with bpm difference below threshold*ratio "
             "are merged (default 1.0; >1 folds near-misses back)",
    )
    args = parser.parse_args()

    wav_path: str | None = None
    try:
        import librosa  # type: ignore

        sys.stderr.write(f"[bpm] decoding {args.input} ...\n")
        wav_path = decode_to_wav(args.input, args.ffmpeg)

        sys.stderr.write("[bpm] loading audio ...\n")
        y, sr = librosa.load(wav_path, sr=None, mono=True)

        sys.stderr.write("[bpm] onset envelope + dynamic tempo ...\n")
        onset_env = librosa.onset.onset_strength(y=y, sr=sr)
        # librosa >= 0.11 moved tempo to librosa.feature.
        dtempo = librosa.feature.tempo(
            onset_envelope=onset_env, sr=sr, aggregate=None
        )
        # feature.tempo may return shape (1, n); flatten to match the envelope.
        if hasattr(dtempo, "ravel"):
            dtempo = dtempo.ravel()
        # dtempo is per-frame; beat_track accepts a matching-length bpm hint.
        _tempo_scalar, beats = librosa.beat.beat_track(
            onset_envelope=onset_env, sr=sr, bpm=dtempo, units="time"
        )
        beat_times = list(beats)

        sys.stderr.write(f"[bpm] {len(beat_times)} beats detected\n")
        segments = build_segments(
            beat_times, args.threshold, args.window, args.merge_ratio
        )
        sys.stderr.write(f"[bpm] {len(segments)} segments\n")

        print(json.dumps({"ok": True, "segments": segments}, ensure_ascii=False))
        return 0
    except Exception as exc:  # noqa: BLE001
        import traceback

        traceback.print_exc(file=sys.stderr)
        print(
            json.dumps(
                {"ok": False, "error": f"{type(exc).__name__}: {exc}"},
                ensure_ascii=False,
            )
        )
        return 1
    finally:
        if wav_path and os.path.exists(wav_path):
            os.unlink(wav_path)


if __name__ == "__main__":
    sys.exit(main())
