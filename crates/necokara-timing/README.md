# necokara-timing

Timing logic for Necokara lyrics.

## What it does

Turns lyric content + checks into concrete times:

- **BPM segments** (`bpm_list`): `BpmList` - a `BTreeMap` of `start -> bpm`
  stretches over the timeline, with a sentinel `bpm 0` default so every point
  in time has a well-defined tempo. O(log n) insert/remove/`bpm_at`.
- **Check interpolation** (`checks_interpolate`): `interpolate_to_word` fills
  cell `start`/`duration` times between anchored checks - main cells evenly,
  ruby per-word with anchor pinning (see module docs for the full rule).

Depends on `necokara-lyrics` for the base types (`CkTime`, streams, word
allocator). Serialization is a separate translation layer.

## Usage sketch

```rust
use necokara_timing::BpmList;
use necokara_lyrics::CkTime;

let mut bpm = BpmList::new();
bpm.insert(CkTime::new(0), 120.0);
assert_eq!(bpm.bpm_at(CkTime::new(500)), 120.0);
```
