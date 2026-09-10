#!/usr/bin/env python3
"""Animation preset: FadeInOut.

Fade the whole line in over the lead-in, hold it for the singing window, then
fade it out over the lead-out.

- ``animation_in_ms``  = ``lead_in_ms`` (clamped to the window)
- ``animation_out_ms`` = ``lead_out_ms`` (clamped so the two never overlap)
- line track: 0 at ``animation_start`` -> 1 at ``line_start``, held until
  ``line_end`` -> 0 at ``animation_end``.

Protocol: see ``develop/DESIGN_v0.4_animation.md`` section 4 and ``_common``.
"""

from __future__ import annotations

import sys

from _common import anim_window, clamp, fade_line_keyframes, line_animation, run_preset


def build_line(line: dict) -> dict:
    start, end = anim_window(line)
    window = end - start

    fade_in = max(0, int(line["lead_in_ms"]))
    fade_out = max(0, int(line["lead_out_ms"]))

    animation_in = clamp(fade_in, 0, window)
    animation_out = clamp(fade_out, 0, window - animation_in)

    return line_animation(
        line,
        animation_in_ms=animation_in,
        animation_out_ms=animation_out,
        line_keyframes=fade_line_keyframes(start, end, fade_in, fade_out),
    )


if __name__ == "__main__":
    sys.exit(run_preset(build_line))
