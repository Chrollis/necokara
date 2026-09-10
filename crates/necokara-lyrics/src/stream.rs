//! Character stream: the ordered list of character cells that forms the
//! backbone of a lyric document (main text stream and ruby stream).
//!
//! Each [`CharCell`] stores one character, an optional `start` time, and a
//! `duration` in milliseconds. `start` is the moment the character begins;
//! `duration` is its length (the UTAU-style fine-tune view needs an explicit
//! length to display/drag the end). A `duration` of 0 means "not set yet".

use crate::ck_char::{CharKind, CkChar};
use crate::ck_time::CkTime;

/// One character cell in a stream.
///
/// `ch` is the raw character; `start` when the character begins (ms);
/// `duration` how long it lasts (ms, 0 = not set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharCell {
    pub ch: char,
    pub start: Option<CkTime>,
    pub duration: u32,
}

impl CharCell {
    /// A cell with a start time (duration 0 = unset).
    pub fn set(ch: char, start: CkTime) -> Self {
        Self {
            ch,
            start: Some(start),
            duration: 0,
        }
    }

    /// A cell with a start time and explicit duration.
    pub fn set_duration(ch: char, start: CkTime, duration: u32) -> Self {
        Self {
            ch,
            start: Some(start),
            duration,
        }
    }

    /// A cell with no timing (untimed).
    pub fn unset(ch: char) -> Self {
        Self {
            ch,
            start: None,
            duration: 0,
        }
    }

    /// Split category of this character (delegates to `CkChar`).
    pub fn kind(&self) -> CharKind {
        CkChar::new(self.ch).kind()
    }

    /// Whether this is a line break.
    pub fn is_new_line(&self) -> bool {
        self.ch == '\n'
    }

    /// Whether this cell has a start time.
    pub fn is_timed(&self) -> bool {
        self.start.is_some()
    }

    /// Shift the start time by an offset (no-op when unset).
    pub fn shift(&mut self, offset: CkTime) {
        if let Some(start) = self.start {
            self.start = Some(start + offset);
        }
    }
}

impl From<char> for CharCell {
    fn from(ch: char) -> Self {
        Self::unset(ch)
    }
}

/// An ordered list of character cells.
///
/// Mirrors the reference `char_stream`: a plain growable vector of cells with
/// a few conveniences (string conversion, slicing, insertion/removal used by
/// editing).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharStream {
    pub cells: Vec<CharCell>,
}

impl CharStream {
    /// New empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from a character string (all cells untimed).
    pub fn from_chars(text: &str) -> Self {
        Self {
            cells: text.chars().map(CharCell::unset).collect(),
        }
    }

    /// Number of cells.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the stream is empty.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Concatenate all characters into a `String`.
    pub fn as_string(&self) -> String {
        self.cells.iter().map(|cell| cell.ch).collect()
    }

    /// Copy the cells covered by any range bound (`a..b`, `a..`, `..b`, `..`).
    pub fn slice(
        &self,
        range: impl std::ops::RangeBounds<usize> + std::slice::SliceIndex<[CharCell], Output = [CharCell]>,
    ) -> Self {
        Self {
            cells: self.cells[range].to_vec(),
        }
    }

    /// Split the stream into lines at `\n` cells.
    ///
    /// Each returned range is a closed interval `[start, end]` over this
    /// stream's cell indices and includes the line's trailing `\n`. A final
    /// line without `\n` ends at the last cell. An empty stream yields no
    /// lines, and a stream ending in `\n` does not produce an extra empty
    /// trailing line.
    pub fn line_ranges(&self) -> Vec<std::ops::RangeInclusive<usize>> {
        let mut lines = Vec::new();
        if self.cells.is_empty() {
            return lines;
        }

        let mut start = 0usize;
        for (index, cell) in self.cells.iter().enumerate() {
            if cell.ch == '\n' {
                lines.push(start..=index);
                start = index + 1;
            }
        }
        // Trailing content without a final `\n`.
        if start < self.cells.len() {
            lines.push(start..=self.cells.len() - 1);
        }
        lines
    }
}

impl std::ops::Deref for CharStream {
    type Target = Vec<CharCell>;
    fn deref(&self) -> &Vec<CharCell> {
        &self.cells
    }
}

impl std::ops::DerefMut for CharStream {
    fn deref_mut(&mut self) -> &mut Vec<CharCell> {
        &mut self.cells
    }
}

impl std::fmt::Display for CharStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compare a `Vec<RangeInclusive>` against inclusive `(start, end)` pairs.
    fn pairs(ranges: Vec<std::ops::RangeInclusive<usize>>) -> Vec<(usize, usize)> {
        ranges.into_iter().map(|r| (*r.start(), *r.end())).collect()
    }

    #[test]
    fn line_ranges_empty() {
        let stream = CharStream::new();
        assert!(stream.line_ranges().is_empty());
    }

    #[test]
    fn line_ranges_no_newline_is_one_line() {
        let stream = CharStream::from_chars("abc");
        assert_eq!(pairs(stream.line_ranges()), vec![(0, 2)]);
    }

    #[test]
    fn line_ranges_include_trailing_newline() {
        // "a\nb\nc" -> cells: a \n b \n c
        let stream = CharStream::from_chars("a\nb\nc");
        assert_eq!(pairs(stream.line_ranges()), vec![(0, 1), (2, 3), (4, 4)]);
    }

    #[test]
    fn line_ranges_stream_ending_with_newline_has_no_extra_line() {
        let stream = CharStream::from_chars("a\n");
        assert_eq!(pairs(stream.line_ranges()), vec![(0, 1)]);

        let only_newline = CharStream::from_chars("\n");
        assert_eq!(pairs(only_newline.line_ranges()), vec![(0, 0)]);
    }

    #[test]
    fn line_ranges_consecutive_newlines_are_empty_lines() {
        // "\n\n" -> two empty lines, each holding one `\n`.
        let stream = CharStream::from_chars("\n\n");
        assert_eq!(pairs(stream.line_ranges()), vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn line_ranges_leading_newline() {
        let stream = CharStream::from_chars("\na");
        assert_eq!(pairs(stream.line_ranges()), vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn line_ranges_start_is_first_cell_of_next_line() {
        let stream = CharStream::from_chars("ab\ncd");
        let ranges = stream.line_ranges();
        assert_eq!(pairs(ranges.clone()), vec![(0, 2), (3, 4)]);
        // The char after a newline starts the next line.
        assert_eq!(*ranges[1].start(), 3);
    }
}
