#!/usr/bin/env python3
"""Animation preset: FadeOut.

Hold the whole line visible from the animation window start, then fade it out
over the lead-out window.

- ``animation_in_ms``  = 0
- ``animation_out_ms`` = ``lead_out_ms`` (clamped to the window)
- line track: opacity 1 held, then 1 at ``line_end`` -> 0 at
  ``animation_end``.

Protocol: see ``develop/DESIGN_v0.4_animation.md`` section 4 and ``_common``.
"""

from __future__ import annotations

import sys

from _common import anim_window, clamp, fade_line_keyframes, line_animation, run_preset


def build_line(line: dict) -> dict:
    start, end = anim_window(line)
    window = end - start

    fade_out = max(0, int(line["lead_out_ms"]))
    animation_out = clamp(fade_out, 0, window)

    return line_animation(
        line,
        animation_in_ms=0,
        animation_out_ms=animation_out,
        line_keyframes=fade_line_keyframes(start, end, 0, fade_out),
    )


if __name__ == "__main__":
    sys.exit(run_preset(build_line))
