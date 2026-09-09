# necokara-lyrics

Lyrics data structures and reading logic for Necokara.

## What it does

`necokara-lyrics` is the in-memory content model of a lyric document: the
character streams that carry the text being sung and its ruby readings, the
word allocation that groups streams into words, and the container that ties
them together.

It does **not** define the on-disk format �?serialization is a separate
translation layer. Timing logic and project-level structures live in sibling
crates (see below).

## Crate layout

```
necokara-lyrics      lyric content: chars, streams, words        �?this crate
necokara-timing   BPM segments, check→time interpolation      (depends on lrc)
necokara-project     metadata (descriptive) + settings (logic)   (depends on lyrics + timing)
```

## Modules

| Module | Purpose |
|---|---|
| `ck_char` | `CkChar`: a char wrapper with the four-way split classification (`CharKind`: per-char / per-word / separator / asyllabic) plus Unicode property wrappers (category, script) |
| `ck_time` | `CkTime`: integer-millisecond time, `CkTimeFormat`, parsing/formatting (`[mm:ss.mmm]` LRC / `[mm:ss:cc]` NKM3) |
| `stream` | `CharCell { ch, start, duration }` and `CharStream` (the ordered cell list; main and ruby) |
| `allocator` | `WordSeg` (main/ruby spans + per-main ruby segmentation + checks) and `WordAlloc` (lazy prefix sums, dirty-rebuild) |
| `lyrics` | `Lyrics { main_stream, ruby_stream, word_allocator }` container + `ok()` consistency checks (last word is a lone `\n`) |

## Model in one picture

```
Lyrics
├── main_stream    CharCell{ ch, start?, duration }   // the text being sung
├── ruby_stream    CharCell{ ... }                    // readings
└── word_allocator WordAlloc
      └── words: WordSeg{ main_len, ruby_seg_lens[], checks[] }
```

- A **word** owns a span of main characters and a span of ruby characters;
  the ruby is split into per-main-character segments via `ruby_seg_lens`.
- **Checks** live at word level: `check_kind` (down/up) + `checks: Vec<Option<CkTime>>`
  (length = check count; `None` = placed but not yet timed).
- The document always ends with a lone `\n` word (mirroring the C++ reference).

## Usage sketch

```rust
use necokara_lyrics::{CharStream, WordAlloc, WordSeg, Lyrics, CkTime};

let lyrics = Lyrics {
    main_stream: CharStream::from_chars("hello\n"),
    ruby_stream: CharStream::from_chars(""),
    word_allocator: WordAlloc::new(),
};
assert!(lyrics.ok()); // ...once the word allocator matches the streams
```
