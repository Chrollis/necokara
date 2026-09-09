//! Necokara timing logic: BPM segments, check-to-time interpolation, and the
//! python wrappers for BPM detection / vocal separation / lyric alignment.
//!
//! Depends on necokara-lyrics for the base types (`CkTime`, streams, the word
//! allocator) and necokara-error for unified errors.

pub mod align_apply;
pub mod bpm_list;
pub mod checks_interpolate;
pub mod lyrics_text;
pub mod separate_align;

pub use align_apply::{align_apply_main_text, align_apply_reading_text};
pub use bpm_list::{BpmList, BPM_NONE, SENTINEL_START};
pub use checks_interpolate::{
    interpolate_from_word_first, interpolate_from_word_last, interpolate_to_word,
};
pub use lyrics_text::{lyrics_main_text_plain, lyrics_reading_text_plain};
pub use separate_align::{
    ai_languages, align, bpm_detect, separate, AlignChar, SeparateOutput,
};
