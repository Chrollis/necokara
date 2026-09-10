#!/usr/bin/env python3
"""Shared helpers for the animation preset scripts.

Protocol (see develop/DESIGN_v0.4_animation.md section 4):

- stdin  : one UTF-8 JSON document ``{"fps": float, "lines": [...]}``
- stdout : one UTF-8 JSON document ``{"ok": true, "lines": [...]}`` on success
           or ``{"ok": false, "error": "..."}`` on failure
- stderr : free-form diagnostics

The scripts are pure calculations: no project state, no lyric text, only
indices and times. Read/write through the byte streams so the system code page
never matters.

Line geometry (all absolute milliseconds):

    animation_start = line_start_ms - lead_in_ms
    animation_end   = line_end_ms   + lead_out_ms

``lead_in_ms`` / ``lead_out_ms`` are inputs; a script only decides
``animation_in_ms`` / ``animation_out_ms`` (the first four scripts derive them
from the lead-in / lead-out, so ``idle = line_end - line_start`` or 0).
"""

from __future__ import annotations

import json
import sys

# Fade length used per character when the caller does not give a shorter one.
DEFAULT_CHAR_FADE_MS = 150


# ---------------------------------------------------------------------------
# Protocol
# ---------------------------------------------------------------------------


def read_request() -> dict:
    """Read the request document from stdin (UTF-8)."""
    raw = sys.stdin.buffer.read()
    return json.loads(raw.decode("utf-8"))


def emit_ok(lines: list[dict]) -> None:
    """Write a successful response."""
    _emit({"ok": True, "lines": lines})


def emit_error(message: str) -> None:
    """Write a failed response."""
    _emit({"ok": False, "error": message})


def _emit(payload: dict) -> None:
    sys.stdout.buffer.write(json.dumps(payload, ensure_ascii=False).encode("utf-8"))
    sys.stdout.buffer.write(b"\n")
    sys.stdout.buffer.flush()


# ---------------------------------------------------------------------------
# State / keyframe helpers
# ---------------------------------------------------------------------------


def identity_transform() -> dict:
    """A full identity ``Transform3D`` document."""
    return {
        "translate_x_px": 0.0,
        "translate_y_px": 0.0,
        "translate_z_px": 0.0,
        "scale_x": 1.0,
        "scale_y": 1.0,
        "scale_z": 1.0,
        "rotate_x_deg": 0.0,
        "rotate_y_deg": 0.0,
        "rotate_z_deg": 0.0,
        "skew_x_deg": 0.0,
        "skew_y_deg": 0.0,
        "origin_x": 0.5,
        "origin_y": 0.5,
        "origin_z": 0.5,
        "perspective_px": None,
    }


def state(opacity: float = 1.0, visible: bool = True, transform: dict | None = None) -> dict:
    """A full ``AnimatedState`` document.

    Every field is written, so the reader never needs implicit defaults.
    """
    return {
        "visible": bool(visible),
        "opacity": float(opacity),
        "transform": transform if transform is not None else identity_transform(),
    }


def keyframe(time_ms: int, opacity: float, easing: str = "linear") -> dict:
    """One state keyframe."""
    return {
        "time_ms": int(time_ms),
        "state": state(opacity=opacity),
        "easing": easing,
    }


def build_keyframes(points: list[tuple[int, float]]) -> list[dict]:
    """Build a linear keyframe track from ``(time_ms, opacity)`` control points.

    Points sharing a time collapse (the later entry wins). The result is
    strictly ascending, as the Rust side requires. A lone point is extended
    into a two-point hold, because a single keyframe only applies at that
    instant and the renderer would inherit the parent state elsewhere.
    """
    ordered: dict[int, float] = {}
    for time_ms, opacity in points:
        ordered[int(time_ms)] = float(opacity)

    times = sorted(ordered)
    if not times:
        raise ValueError("no keyframes were produced")
    if len(times) == 1:
        only = times[0]
        ordered[only + 1] = ordered[only]
        times = [only, only + 1]

    return [keyframe(time, ordered[time]) for time in times]


# ---------------------------------------------------------------------------
# Line geometry
# ---------------------------------------------------------------------------


def anim_window(line: dict) -> tuple[int, int]:
    """The full animation window ``(start_ms, end_ms)`` of one line."""
    start = int(line["line_start_ms"]) - int(line["lead_in_ms"])
    end = int(line["line_end_ms"]) + int(line["lead_out_ms"])
    if end <= start:
        # Degenerate input (zero-length line with no lead): keep a sampling
        # window so the renderer can still evaluate the track.
        end = start + 1
    return start, end


def fade_line_keyframes(
    start_ms: int,
    end_ms: int,
    fade_in_ms: int,
    fade_out_ms: int,
) -> list[dict]:
    """Whole-line fade in / out.

    Durations are clamped to the window and to each other, so the two fades
    never overlap in the first four scripts (the wire format itself can express
    overlap; a richer script can emit more keyframes). A zero duration means
    "no fade on that side".
    """
    window = end_ms - start_ms
    fade_in_ms = max(0, min(int(fade_in_ms), window))
    fade_out_ms = max(0, min(int(fade_out_ms), window - fade_in_ms))

    in_end = start_ms + fade_in_ms
    out_start = end_ms - fade_out_ms

    points: list[tuple[int, float]] = []
    if fade_in_ms > 0:
        points.append((start_ms, 0.0))
        points.append((in_end, 1.0))
    else:
        points.append((start_ms, 1.0))

    if fade_out_ms > 0:
        if out_start > in_end:
            points.append((out_start, 1.0))
        points.append((end_ms, 0.0))
    elif end_ms > points[-1][0]:
        points.append((end_ms, 1.0))

    return build_keyframes(points)


def char_fade_keyframes(
    window_start: int,
    window_end: int,
    cell_start_ms: int,
    duration_ms: int = 0,
) -> list[dict] | None:
    """Per-character fade-in: fully visible at the character's own start.

    Returns ``None`` when the fade would be degenerate; the caller then omits
    the track and the character inherits the line state (fully visible).
    """
    fade_end = min(max(int(cell_start_ms), window_start), window_end)

    fade_duration = DEFAULT_CHAR_FADE_MS
    if duration_ms and int(duration_ms) > 0:
        fade_duration = min(fade_duration, int(duration_ms))

    fade_start = max(window_start, fade_end - fade_duration)
    if fade_start >= fade_end:
        return None
    return build_keyframes([(fade_start, 0.0), (fade_end, 1.0)])


def line_animation(
    line: dict,
    animation_in_ms: int,
    animation_out_ms: int,
    line_keyframes: list[dict],
    main: list[dict] | None = None,
    ruby: list[dict] | None = None,
) -> dict:
    """Assemble one ``LineAnimation`` document, echoing index and script."""
    return {
        "line_index": int(line["line_index"]),
        "script": str(line["script"]),
        "animation_in_ms": int(animation_in_ms),
        "animation_out_ms": int(animation_out_ms),
        "line": line_keyframes,
        "main": main if main is not None else [],
        "ruby": ruby if ruby is not None else [],
        "decors": [],
    }


def clamp(value: int, low: int, high: int) -> int:
    """Clamp ``value`` into ``[low, high]``."""
    return max(low, min(value, high))


def run_preset(build_line) -> int:
    """Read the request, build every line, and emit the response.

    ``build_line`` maps one request line to one ``LineAnimation`` document.
    """
    try:
        request = read_request()
        lines = request.get("lines", [])
        results = [build_line(line) for line in lines]
        emit_ok(results)
        return 0
    except Exception as exc:  # noqa: BLE001
        emit_error(f"{type(exc).__name__}: {exc}")
        return 1
