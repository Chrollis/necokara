//! Necokara lyrics data structures and reading logic.
//!
//! This crate holds the lyric-content model: character streams (main text +
//! ruby), the word allocation over them, and the container that ties them
//! together. Timing logic (BPM, check interpolation) lives in necokara-timing;
//! project metadata/settings live in necokara-project.
//!
//! Naming uses the `ck` prefix domain (`ck_char`, `ck_time`, Rust types
//! are `CamelCase` (`CkChar`), everything else `snake_case`.

pub mod allocator;
pub mod ck_char;
pub mod ck_time;
pub mod lyrics;
pub mod stream;

pub use allocator::{CheckKind, WordAlloc, WordSeg};
pub use ck_char::{CharKind, CkChar};
pub use ck_time::{format_time, parse_time, CkTime, CkTimeFormat};
pub use lyrics::Lyrics;
pub use stream::{CharCell, CharStream};
