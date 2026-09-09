//! Lyrics -> plain text for alignment.
//!
//! Stable-ts force-aligns a plain text onto audio. These helpers turn a
//! [`Lyrics`] into the two candidate texts:
//!  - the plain sung text (main stream), and
//!  - a "reading" text where words that carry ruby use their ruby reading.
//!
//! Newlines in the main stream are preserved in both, so per-line alignment
//! keeps working.

use necokara_lyrics::lyrics::Lyrics;

/// The plain sung text: the whole main stream as a string (newlines kept).
pub fn lyrics_main_text_plain(lyrics: &Lyrics) -> String {
    lyrics.main_stream.as_string()
}

/// The reading text: for every word, if it has ruby (`ruby_len != 0`) use its
/// ruby-stream slice, otherwise use its main-stream slice. Newlines are kept
/// (the trailing `\n` word has no ruby and falls back to main).
pub fn lyrics_reading_text_plain(lyrics: &Lyrics) -> String {
    let mut alloc = lyrics.word_allocator.clone();
    // Pre-fetch per-word ruby flags to avoid borrow conflicts with the
    // mutable range queries.
    let ruby_lens: Vec<usize> = alloc.words().iter().map(|w| w.ruby_len).collect();
    let mut out = String::new();

    for (wi, &ruby_len) in ruby_lens.iter().enumerate() {
        if ruby_len != 0 {
            let range = alloc.ruby_range(wi);
            for cell in &lyrics.ruby_stream.cells[range] {
                out.push(cell.ch);
            }
        } else {
            let range = alloc.main_range(wi);
            for cell in &lyrics.main_stream.cells[range] {
                out.push(cell.ch);
            }
        }
    }
    out
}
