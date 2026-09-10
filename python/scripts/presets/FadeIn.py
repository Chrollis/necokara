#!/usr/bin/env python3
"""Animation preset: FadeIn.

Fade the whole line in over the lead-in window, then hold it visible until the
animation window ends.

- ``animation_in_ms``  = ``lead_in_ms`` (clamped to the window)
- ``animation_out_ms`` = 0
- line track: opacity 0 at ``animation_start`` -> 1 at ``line_start``, held.

Protocol: see ``develop/DESIGN_v0.4_animation.md`` section 4 and ``_common``.
"""

from __future__ import annotations

import sys

from _common import anim_window, clamp, fade_line_keyframes, line_animation, run_preset


def build_line(line: dict) -> dict:
    start, end = anim_window(line)
    window = end - start

    fade_in = max(0, int(line["lead_in_ms"]))
    animation_in = clamp(fade_in, 0, window)

    return line_animation(
        line,
        animation_in_ms=animation_in,
        animation_out_ms=0,
        line_keyframes=fade_line_keyframes(start, end, fade_in, 0),
    )


if __name__ == "__main__":
    sys.exit(run_preset(build_line))
