#!/usr/bin/env python3
"""Animation preset: PerCharFade.

Per-character staggered fade-in plus a whole-line fade-out.

- Each main / ruby character fades in (opacity 0 -> 1) and is fully visible at
  its own ``start_ms``; the fade length is ``min(150 ms, cell duration)`` and is
  clamped so it never starts before the animation window. Characters whose fade
  is degenerate are omitted and inherit the line state.
- The line itself holds opacity 1 and fades out over the lead-out, so the line
  disappears at ``animation_end``.
- ``animation_in_ms``  = ``line_end_ms - animation_start`` (the per-character
  fades span the singing window)
- ``animation_out_ms`` = ``lead_out_ms``

That gives ``idle = 0``: the staggered fades cover the whole window.

Note: ruby characters use their own ``start_ms``; the script does not need to
know which main character owns them.

Protocol: see ``develop/DESIGN_v0.4_animation.md`` section 4 and ``_common``.
"""

from __future__ import annotations

import sys

from _common import (
    anim_window,
    char_fade_keyframes,
    clamp,
    fade_line_keyframes,
    line_animation,
    run_preset,
)


def build_line(line: dict) -> dict:
    start, end = anim_window(line)
    window = end - start
    line_end = int(line["line_end_ms"])

    lead_out = max(0, int(line["lead_out_ms"]))
    animation_in = clamp(line_end - start, 0, window)
    animation_out = clamp(lead_out, 0, window - animation_in)

    main = []
    for cell in line.get("main", []):
        keyframes = char_fade_keyframes(
            start,
            end,
            int(cell["start_ms"]),
            int(cell.get("duration_ms", 0)),
        )
        if keyframes is not None:
            main.append({"char_index": int(cell["char_index"]), "keyframes": keyframes})

    ruby = []
    for cell in line.get("ruby", []):
        keyframes = char_fade_keyframes(
            start,
            end,
            int(cell["start_ms"]),
            int(cell.get("duration_ms", 0)),
        )
        if keyframes is not None:
            ruby.append({"ruby_index": int(cell["ruby_index"]), "keyframes": keyframes})

    return line_animation(
        line,
        animation_in_ms=animation_in,
        animation_out_ms=animation_out,
        line_keyframes=fade_line_keyframes(start, end, 0, lead_out),
        main=main,
        ruby=ruby,
    )


if __name__ == "__main__":
    sys.exit(run_preset(build_line))
