//! Lyric document body: the two character streams plus their word
//! allocation.
//!
//! `Lyrics` is the container that ties the main text stream, the ruby stream,
//! and the word allocator together. The word allocator describes how the
//! streams are grouped into words (main spans + ruby segmentation + checks).

use crate::allocator::WordAlloc;
use crate::stream::CharStream;

/// The lyric body: main/ruby character streams + the word allocation over
/// them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lyrics {
    /// Main text stream (the characters being sung, including `\n`).
    pub main_stream: CharStream,
    /// Ruby (reading) stream.
    pub ruby_stream: CharStream,
    /// Word allocation over both streams.
    pub word_allocator: WordAlloc,
}

impl Lyrics {
    /// New empty lyrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the structure is self-consistent:
    ///  - the total main length from the word allocator matches `main_stream`,
    ///    and the total ruby length matches `ruby_stream`;
    ///  - the last word is a single `\n` character (main_len == 1, no ruby),
    ///    and `main_stream` therefore ends with `\n` — mirroring the C++
    ///    reference where the document always ends with a newline.
    pub fn ok(&self) -> bool {
        let words = self.word_allocator.words();
        let total_main: usize = words.iter().map(|w| w.main_len).sum();
        let total_ruby: usize = words.iter().map(|w| w.ruby_len).sum();
        if total_main != self.main_stream.len() || total_ruby != self.ruby_stream.len() {
            return false;
        }

        // Last word must be a lone newline: main_len == 1, no ruby.
        match words.last() {
            Some(w) if w.main_len == 1 && w.ruby_len == 0 => {}
            _ => return false,
        }
        // ...and the last cell of main_stream must be '\n'.
        match self.main_stream.cells.last() {
            Some(cell) => cell.ch == '\n' && cell.is_new_line(),
            None => false,
        }
    }

    /// Split the main stream into lines (see [`CharStream::line_ranges`]).
    ///
    /// Each range is a closed interval over main-stream cell indices and
    /// includes the line's trailing `\n`.
    pub fn line_ranges(&self) -> Vec<std::ops::RangeInclusive<usize>> {
        self.main_stream.line_ranges()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allocator::{WordAlloc, WordSeg};
    use crate::stream::CharStream;

    /// Consistent sample: main "明日\n" (3 cells), ruby "あし" (2 cells),
    /// words = [{main 2, ruby 2, segs [1,1]}, {main 1 (\n), ruby 0}].
    fn consistent() -> Lyrics {
        Lyrics {
            main_stream: CharStream::from_chars("明日\n"),
            ruby_stream: CharStream::from_chars("あし"),
            word_allocator: WordAlloc::from_words(vec![
                WordSeg::with_ruby(2, vec![1, 1]),
                WordSeg::plain(1),
            ]),
        }
    }

    #[test]
    fn ok_on_consistent() {
        assert!(consistent().ok());
    }

    #[test]
    fn main_len_mismatch_fails() {
        let mut l = consistent();
        l.main_stream = CharStream::from_chars("明日日\n"); // one extra cell
        assert!(!l.ok());
    }

    #[test]
    fn ruby_len_mismatch_fails() {
        let mut l = consistent();
        l.ruby_stream = CharStream::from_chars("あした"); // one extra ruby
        assert!(!l.ok());
    }

    #[test]
    fn last_word_not_newline_fails() {
        let mut l = consistent();
        // Last word (index 1) becomes a 2-char word -> not a lone \n.
        l.word_allocator = WordAlloc::from_words(vec![
            WordSeg::with_ruby(2, vec![1, 1]),
            WordSeg::plain(2),
        ]);
        assert!(!l.ok());
    }

    #[test]
    fn last_word_has_ruby_fails() {
        let mut l = consistent();
        // Last (\n) word wrongly carries ruby; total lengths still match so
        // only the "last word must be a lone \n" rule trips.
        l.word_allocator = WordAlloc::from_words(vec![
            WordSeg::with_ruby(2, vec![1, 1]),
            WordSeg::with_ruby(1, vec![1]),
        ]);
        l.ruby_stream = CharStream::from_chars("あしか");
        assert!(!l.ok());
    }

    #[test]
    fn missing_trailing_newline_fails() {
        let mut l = consistent();
        l.main_stream = CharStream::from_chars("明日"); // no \n
        l.word_allocator = WordAlloc::from_words(vec![WordSeg::with_ruby(2, vec![1, 1])]);
        assert!(!l.ok());
    }

    #[test]
    fn line_ranges_delegates_to_main_stream() {
        let mut l = consistent();
        // "明日\n" is a single line covering cells 0..=2, including the `\n`.
        let ranges = l.line_ranges();
        assert_eq!(ranges.len(), 1);
        assert_eq!((*ranges[0].start(), *ranges[0].end()), (0, 2));

        // Two lines: "明日\n" + "花\n".
        l.main_stream = CharStream::from_chars("明日\n花\n");
        let ranges = l.line_ranges();
        assert_eq!(ranges.len(), 2);
        assert_eq!((*ranges[0].start(), *ranges[0].end()), (0, 2));
        assert_eq!((*ranges[1].start(), *ranges[1].end()), (3, 4));
    }
}
