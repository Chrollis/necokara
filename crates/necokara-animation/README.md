# necokara-animation

Line-level preset animation and decor specs for Necokara.

## What it does

`necokara-animation` is the animation layer of the core data model. It defines
*what* a line's animation looks like over time; a Python script computes the
per-line keyframes, and the WebGL renderer evaluates them at playback time and
draws. This crate never rasterizes pixels and never carries lyric text — only
indices and states.

```text
lyrics + line_index + script
        │  python/scripts/presets (pure calculation)
        ▼
  LineAnimation (sparse keyframes)
        │  IPC
        ▼
  WebGL renderer (evaluate + draw)
```

## Scope

In scope:

- the dynamic animation script set and its per-line allocation;
- 3D transforms and animation states;
- sparse keyframe tracks for line / main character / ruby character;
- grouping lines into script batches and invoking the Python presets;
- the JSON wire format between Rust and the scripts;
- renderer-interpreted decor instructions (container only for now).

Out of scope by design:

- clip / composition track / timeline trimming (composition layer);
- per-frame rendering or pixel drawing;
- karaoke wipe (that is the dual-state fill in `necokara-style`);
- style definitions (font / fill / outline / shadow live in `necokara-style`).

The animation script set is **data-driven**: `manifest.json` is the
authoritative list, so there is no hard-coded preset enum in Rust. An
allocation stores the script name (file name without `.py`).

## Modules

| Module | Purpose |
|---|---|
| `alloc` | `LineAnimRun` / `LineAnimAlloc`: sorted, non-overlapping runs carrying a script name plus `lead_in_ms` / `lead_out_ms` |
| `transform` | `Transform3D`: translation, scale, 3-axis rotation, skew, normalized origin, optional perspective |
| `state` | `AnimatedState { visible, opacity, transform }` and the evaluated `AnimatedLine` / `AnimatedMain` / `AnimatedRuby` hierarchy |
| `easing` | `Easing`: linear, quadratic in/out/in-out, explicit cubic bezier |
| `keyframe` | `StateKeyframe`, `MainKeyframes`, `RubyKeyframes`, `LineAnimation`, and `sample_state` |
| `decor` | `DecorAnchor`, `DecorValue`, `DecorSpec`, `DecorTarget`, `DecorParamTrack`, `DecorItem`, `AnimatedDecor` |
| `script_manifest` | `AnimationScript` + `load_script_manifest` / `parse_script_manifest` / `script_name_of`: enumerate the animation scripts declared in `python/scripts/presets/manifest.json` |
| `json` | wire format: `request_value` builds the stdin document, `line_animations_from_value` converts the response back |
| `script` | `LineInputs` / `ScriptLineInput` / `group_by_script` / `run_script` / `run_groups`: bucket lines by script and invoke it through `necokara-spawn` |

## Running scripts

```rust
use necokara_animation::{group_by_script, run_groups, load_script_manifest};

// `lines` comes from the application layer (lyrics + timing, no text).
let groups = group_by_script(&alloc, lines);
let manifest = load_script_manifest(scripts_dir)?;
let animations = run_groups(python, scripts_dir, &manifest, 30.0, groups)?;
```

- `group_by_script` skips lines with no allocation and sorts groups by script
  name, so call order is stable;
- `run_script` sends one batch per process (`stdin` JSON, `stdout` envelope),
  rejects mixed-script batches, and verifies the response covers exactly the
  requested lines;
- `decors` must be an empty array for now; a non-empty array is reported as
  `necokara-animation.json.unsupported_decors`.


## Animation script manifest

The preset folder (`python/scripts/presets/`) holds a `manifest.json` array:

```json
[
  {
    "name": { "en_US": "Fade In", "ja_JP": "フェードイン" },
    "file_name": "FadeIn.py"
  }
]
```

- `file_name` — the script's identity: a plain `.py` file name (no path
  separators), unique in the manifest. Stable keys used in project files are
  derived from it in code, not hand-written in the manifest;
- `name` — locale code to display text. Locales are dynamic: any locale may be
  present and none is required, so a UI with no translation falls back to the
  script name (`file_name` without `.py`).

`load_script_manifest(scripts_dir)` reads `<scripts_dir>/manifest.json`,
validates it, and keeps only entries whose script file exists — missing scripts
are silently skipped. `parse_script_manifest(json)` does the same validation
without touching the file system. `AnimationScript` also exposes
`script_name()`, `name_for(locale)`, `display_name(locale)`, and
`path_in(scripts_dir)`.

## Keyframes and evaluation

Each `LineAnimation` carries the `script` name that produced it and has three
parallel sparse levels:

| Level | Applies to | Index space |
|---|---|---|
| `line` | the whole line | — |
| `main` | one main character | full main character-stream index |
| `ruby` | one ruby character | full ruby character-stream index |

- Keyframes are sparse and ascending; the line track defines the animation
  window (first to last keyframe). Outside the window the line is not drawn.
- `sample_state` interpolates a segment with the easing stored on its left
  keyframe; `visible` steps, numeric fields interpolate.
- Missing main / ruby tracks (or tracks that do not cover the sampled time)
  inherit the line state. Ruby ownership is resolved by the renderer through
  `WordAlloc`; the animation data does not repeat it.
- `animation_in_ms` / `animation_out_ms` may overlap (`idle < 0`): both apply
  simultaneously, with no clamping or error.

## Decor and glyph ink

The animation layer cannot rasterize glyphs, so decor placement that depends on
glyph ink is expressed symbolically and resolved by the renderer:

```text
Python / animation layer   DecorAnchor (symbolic reference)
        │ JSON
translation layer          passthrough (does not resolve ink)
        │ IPC
WebGL renderer             resolve with glyph atlas + ink metrics
```

- `DecorAnchor`: `Canvas`, `Box`, `InkBox`, `InkOutline`, `InkArea`.
- `DecorTarget`: `Line` / `Main` / `Ruby` (no `Global` — this module is
  per-line). `Line` ink anchors resolve against the line's combined ink.
- `DecorValue` is schema-free (`Number`, `Bool`, `Text`, `Color`, `Numbers`,
  `Colors`, `Anchor`, `Anchors`); `DecorParamTrack` animates one parameter by
  key, overriding the static value when active.
- There is no explicit decor clip: ink anchors naturally land on the ink;
  effects that leave the line box use `Canvas` anchors.

## Usage sketch

```rust
use necokara_animation::{AnimatedState, Easing, LineAnimAlloc, StateKeyframe};

let mut alloc = LineAnimAlloc::new();
alloc.set(0, 2, "FadeIn", 200, 200);
assert_eq!(alloc.script_at(1), Some("FadeIn"));
assert_eq!(alloc.window_at(1), Some((200, 200)));

let state = AnimatedState::default();
let track = vec![
    StateKeyframe { time_ms: 0, state, easing: Easing::Linear },
    StateKeyframe { time_ms: 100, state, easing: Easing::Linear },
];
assert_eq!(necokara_animation::sample_state(&track, 50).map(|s| s.opacity), Some(1.0));
```

## Errors

- `necokara-animation.line_animation.script_empty` — the script name is empty;
- `necokara-animation.line_animation.line_keyframes_empty` — the line track is
  missing, so the animation has no window;
- `necokara-animation.line_animation.keyframes_unsorted` — keyframe times are
  not strictly ascending;
- `necokara-animation.line_animation.duplicate_index` — a main / ruby index
  appears twice in one line animation;
- `necokara-animation.decor.window_inverted` — a decor's `start_ms > end_ms`;
- `necokara-animation.decor.param_keyframes_unsorted` — a decor parameter
  track's keyframes are not strictly ascending;
- `necokara-animation.script_manifest.*` — `read_failed`, `invalid_json`,
  `not_an_array`, `entry_not_object`, `missing_field`, `wrong_type`,
  `empty_name`, `invalid_file_name`, `duplicate_file_name`;
- `necokara-animation.json.*` — `invalid_payload` (response shape mismatch),
  `request_failed`, `unsupported_decors` (decor wire format not supported yet);
- `necokara-animation.script.*` — `mixed_scripts`, `output_mismatch`,
  `unknown_script`;
- `necokara-spawn.process.*` — transport failures (spawn / exit status / stdin /
  non-JSON stdout / `{"ok": false}`).

`LineAnimation::validate` and `DecorItem::validate` report the animation /
decor codes; `parse_script_manifest` reports the manifest codes; the `json` and
`script` modules report the wire / batching codes. A missing script file is not
an error (it is skipped), and absent animation tracks simply inherit the line
state.

Depends on `necokara-error` (unified errors), `necokara-spawn` (script
processes), `necokara-style` (`CkColor`), `serde` / `serde_json` (wire format
and manifest parsing). Core types themselves carry no serde attributes.

## Shipped presets

`python/scripts/presets/` ships four scripts plus a shared `_common.py` helper.
`animation_in_ms` / `animation_out_ms` are derived from the input window
(`lead_in_ms` / `lead_out_ms`, clamped so the fades never overlap), so `idle`
is either the singing window or 0:

| Script | Line track | `animation_in_ms` | `animation_out_ms` | `idle` |
|---|---|---|---|---|
| `FadeIn` | 0 at `animation_start` → 1 at `line_start`, held | `lead_in_ms` | 0 | singing window |
| `FadeOut` | 1 held → 0 at `animation_end` | 0 | `lead_out_ms` | singing window |
| `FadeInOut` | fade in at the start, fade out at the end | `lead_in_ms` | `lead_out_ms` | singing window |
| `PerCharFade` | 1 held, then fade out over the lead-out | `line_end_ms - animation_start` | `lead_out_ms` | 0 |

`PerCharFade` additionally emits one fade-in track per main and ruby character:
each character is fully visible at its own `start_ms` and fades in over
`min(150 ms, cell duration)`, clamped to the animation window. Characters whose
fade would be degenerate are omitted and inherit the line state.

Ruby uses its own `start_ms`; the scripts do not need to know which main
character owns which ruby character.

