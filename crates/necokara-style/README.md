# necokara-style

Style definitions, applications, and inheritance resolution for Necokara.

## What it does

`necokara-style` models how lyric/document content looks, independent of the
lyrics content model itself:

- **Definitions**: named character, ruby layout, page layout, and paragraph
  (line) styles, plus built-in defaults.
- **Applications**: which style id is applied to which stretch of the document,
  stored as sorted, non-overlapping runs.
- **Resolution**: follows each style's `based_on` chain and fills missing
  fields from the built-in default, returning `CkError` on an incomplete
  default or an inheritance cycle.

Style definitions are intentionally split from style application: definitions
live in `StyleBook`, applications live in `StyleAlloc`, and `StyleManager`
combines both with defaults for resolved lookup.

## Style kinds

- **Character style** (`character`) - the visual look of one sung lyric
  character:
  - CJK and Western font families, size, weight, italic, letter spacing,
    baseline shift;
  - dual-state karaoke fill (`DualFill`: unsung / sung);
  - wipe direction and transition;
  - optional static outline and shadow, each with its own `DualFill`.
  - Ruby text is rendered with its own character style, not by inheriting the
    base character style.
- **Ruby style** (`ruby`) - ruby layout only: position (auto/above/below),
  horizontal align (auto/left/center/right/space-between/space-around), and
  distance from the base text.
- **Page style** (`page`) - page / text-box layout: text direction, margins,
  vertical alignment, line spacing, and optional per-line extra offsets.
- **Paragraph style** (`paragraph`) - one lyric line's horizontal layout:
  `align` (start/center/end), `left_indent_px`, `right_indent_px`. Line
  spacing is owned by `PageStyle`.

Character and page sizes are absolute pixels. Canvas/page dimensions stay in
`necokara-project` settings, not in style definitions.

## Application axes

`StyleAlloc` stores runs over five index axes:

| Axis | Index space | Style kind |
|---|---|---|
| `main_char_runs` | main character stream cells | `CharacterStyle` |
| `ruby_char_runs` | ruby character stream cells | `CharacterStyle` |
| `word_ruby_runs` | word indices | `RubyStyle` |
| `line_para_runs` | line indices | `ParagraphStyle` |
| `page_runs` | line index ranges | `PageStyle` |

Runs are always sorted by `start` and non-overlapping. Setting a style range
overwrites overlapping applications; adjacent runs with the same style id are
merged. Empty ranges are no-ops, and clearing a range splits runs that straddle
its edges.

## Scope

This crate is pure data structures and resolution logic. It deliberately does
**not**:

- mutate `Lyrics` or `WordAlloc`; keeping style applications in sync with
  lyric edits belongs to a higher-level editing/session wrapper;
- serialize to JSON / `.ckp` (translation layer concern);
- render or rasterize anything;
- manage canvas dimensions (project settings) or animation/effect stacks.

`Fill::image` validates its `crl://` reference with `necokara-path`.

## Modules

| Module | Purpose |
|---|---|
| `color` | `CkColor` RGBA color |
| `fill` | `Fill` (solid / linear gradient / linear segments / `crl://` image) and `DualFill` |
| `character` | `CharacterStyle`, `CkOutline`, `CkShadow`, `Wipe`, and `resolve` |
| `ruby` | `RubyStyle`, `RubyPosition`, `RubyAlign`, and `resolve` |
| `page` | `PageStyle`, `PageMargins`, `LineOffset`, `PageTextDirection`, `VerticalAlign`, and `resolve` |
| `paragraph` | `ParagraphStyle`, `ParagraphAlign`, and `resolve` |
| `book` | `StyleBook`: the four named-style tables |
| `alloc` | `StyleAlloc` / `StyleRun`: sorted style application runs |
| `manager` | `StyleManager`: book + defaults + allocation with `resolve_*_at` queries |

## Usage sketch

```rust
use necokara_style::StyleAlloc;

let mut alloc = StyleAlloc::new();
alloc.set_main_char_style(0, 3, "style_char_verse");
alloc.set_line_paragraph_style(0, 2, "style_para_chorus");
alloc.set_page_style(0, 8, "style_page_verse");

assert_eq!(alloc.main_char_style_id_at(1), Some("style_char_verse"));
assert_eq!(alloc.line_paragraph_style_id_at(1), Some("style_para_chorus"));
assert_eq!(alloc.page_style_id_at(7), Some("style_page_verse"));
assert_eq!(alloc.page_style_id_at(8), None);
```

`StyleManager` wraps this with `StyleBook` and built-in defaults so callers can
ask for the effective style directly:

```rust
let style = manager.resolve_main_char_at(1)?;
```

## Errors

Each style kind reports:

- `necokara-style.<kind>.default_incomplete` - the built-in default is missing
  a required field;
- `necokara-style.<kind>.based_on_cycle` - the `based_on` chain contains a
  cycle.

`StyleManager` additionally reports
`necokara-style.manager.style_not_found` when an applied style id is not
present in the `StyleBook`. A missing `based_on` target is treated as the
built-in default and is not an error.
