//! Style allocation: which style id is applied to which stretch of the
//! document.
//!
//! Applications are stored as sorted, non-overlapping runs. Each axis has its
//! own run list:
//!
//! | Axis | Index space | Style kind |
//! |---|---|---|
//! | `main_char_runs` | main character stream cells | `CharacterStyle` |
//! | `ruby_char_runs` | ruby character stream cells | `CharacterStyle` |
//! | `word_ruby_runs` | word indices | `RubyStyle` |
//! | `line_para_runs` | line indices | `ParagraphStyle` |
//! | `page_runs` | line index ranges | `PageStyle` |
//!
//! The allocation layer stores ids only; definitions live in
//! [`StyleBook`](crate::StyleBook) and resolution lives in
//! [`StyleManager`](crate::StyleManager).

use std::ops::Range;

/// One sorted, non-overlapping style application run over an index axis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleRun {
    /// First covered index (inclusive).
    pub start: usize,
    /// End index (exclusive).
    pub end: usize,
    /// Applied style id.
    pub style_id: String,
}

impl StyleRun {
    /// Build a style run.
    pub fn new(start: usize, end: usize, style_id: impl Into<String>) -> Self {
        Self {
            start,
            end,
            style_id: style_id.into(),
        }
    }

    /// Whether the run covers `index`.
    pub fn contains(&self, index: usize) -> bool {
        self.start <= index && index < self.end
    }

    /// Number of covered indices.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the run is empty.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// Style allocation manager.
///
/// All run vectors are kept sorted by `start` and non-overlapping; setting a
/// style range overwrites overlapping applications. Adjacent runs with the
/// same style id are merged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleAlloc {
    main_char_runs: Vec<StyleRun>,
    ruby_char_runs: Vec<StyleRun>,
    word_ruby_runs: Vec<StyleRun>,
    line_para_runs: Vec<StyleRun>,
    page_runs: Vec<StyleRun>,
}

impl StyleAlloc {
    /// Build an empty allocation.
    pub fn new() -> Self {
        Self::default()
    }

    // ---- raw run accessors -------------------------------------------------

    /// Main character style runs.
    pub fn main_char_runs(&self) -> &[StyleRun] {
        &self.main_char_runs
    }

    /// Ruby character style runs.
    pub fn ruby_char_runs(&self) -> &[StyleRun] {
        &self.ruby_char_runs
    }

    /// Word-level ruby layout style runs.
    pub fn word_ruby_runs(&self) -> &[StyleRun] {
        &self.word_ruby_runs
    }

    /// Line-level paragraph style runs.
    pub fn line_para_runs(&self) -> &[StyleRun] {
        &self.line_para_runs
    }

    /// Page style runs (line index ranges).
    pub fn page_runs(&self) -> &[StyleRun] {
        &self.page_runs
    }

    // ---- queries -----------------------------------------------------------

    /// Character style id applied at a main-stream index.
    pub fn main_char_style_id_at(&self, index: usize) -> Option<&str> {
        style_id_at(&self.main_char_runs, index)
    }

    /// Character style id applied at a ruby-stream index.
    pub fn ruby_char_style_id_at(&self, index: usize) -> Option<&str> {
        style_id_at(&self.ruby_char_runs, index)
    }

    /// Ruby layout style id applied at a word index.
    pub fn word_ruby_style_id_at(&self, word_index: usize) -> Option<&str> {
        style_id_at(&self.word_ruby_runs, word_index)
    }

    /// Paragraph style id applied at a line index.
    pub fn line_paragraph_style_id_at(&self, line_index: usize) -> Option<&str> {
        style_id_at(&self.line_para_runs, line_index)
    }

    /// Page style id applied at a line index.
    pub fn page_style_id_at(&self, line_index: usize) -> Option<&str> {
        style_id_at(&self.page_runs, line_index)
    }

    // ---- main character styles --------------------------------------------

    /// Apply a character style to a main-stream range `[start, end)`.
    pub fn set_main_char_style(&mut self, start: usize, end: usize, style_id: impl Into<String>) {
        set_style_run(&mut self.main_char_runs, start..end, style_id.into());
    }

    /// Clear main-stream style applications in `[start, end)`.
    pub fn clear_main_char_style(&mut self, start: usize, end: usize) {
        clear_style_run(&mut self.main_char_runs, start..end);
    }

    // ---- ruby character styles --------------------------------------------

    /// Apply a character style to a ruby-stream range `[start, end)`.
    pub fn set_ruby_char_style(&mut self, start: usize, end: usize, style_id: impl Into<String>) {
        set_style_run(&mut self.ruby_char_runs, start..end, style_id.into());
    }

    /// Clear ruby-stream style applications in `[start, end)`.
    pub fn clear_ruby_char_style(&mut self, start: usize, end: usize) {
        clear_style_run(&mut self.ruby_char_runs, start..end);
    }

    // ---- word ruby layout styles ------------------------------------------

    /// Apply a ruby layout style to a word range `[start, end)`.
    pub fn set_word_ruby_style(&mut self, start: usize, end: usize, style_id: impl Into<String>) {
        set_style_run(&mut self.word_ruby_runs, start..end, style_id.into());
    }

    /// Clear word-level ruby style applications in `[start, end)`.
    pub fn clear_word_ruby_style(&mut self, start: usize, end: usize) {
        clear_style_run(&mut self.word_ruby_runs, start..end);
    }

    // ---- line paragraph styles --------------------------------------------

    /// Apply a paragraph style to a line range `[start, end)`.
    pub fn set_line_paragraph_style(
        &mut self,
        start: usize,
        end: usize,
        style_id: impl Into<String>,
    ) {
        set_style_run(&mut self.line_para_runs, start..end, style_id.into());
    }

    /// Clear paragraph style applications in `[start, end)`.
    pub fn clear_line_paragraph_style(&mut self, start: usize, end: usize) {
        clear_style_run(&mut self.line_para_runs, start..end);
    }

    // ---- page styles -------------------------------------------------------

    /// Apply a page style to a line range `[start, end)`.
    pub fn set_page_style(&mut self, start: usize, end: usize, style_id: impl Into<String>) {
        set_style_run(&mut self.page_runs, start..end, style_id.into());
    }

    /// Clear page style applications in `[start, end)`.
    pub fn clear_page_style(&mut self, start: usize, end: usize) {
        clear_style_run(&mut self.page_runs, start..end);
    }
}

/// Find the style id covering `index`, if any.
fn style_id_at(runs: &[StyleRun], index: usize) -> Option<&str> {
    // `partition_point` gives the first run whose start is greater than index;
    // the run immediately before it may still cover index.
    let pos = runs.partition_point(|run| run.start <= index);
    if pos == 0 {
        return None;
    }
    let run = &runs[pos - 1];
    if index < run.end {
        Some(&run.style_id)
    } else {
        None
    }
}

/// Overwrite `range` with one style run.
fn set_style_run(runs: &mut Vec<StyleRun>, range: Range<usize>, style_id: String) {
    if range.start >= range.end {
        return;
    }

    // Remove or split any run overlapping the new range.
    clear_style_run(runs, range.clone());

    // Insert the new run at the correct sorted position. Existing runs cannot
    // overlap it after the clear step.
    let pos = runs.partition_point(|run| run.start < range.start);
    runs.insert(pos, StyleRun::new(range.start, range.end, style_id));
    merge_adjacent(runs);
}

/// Remove `range` from the run list, splitting runs that straddle its edges.
fn clear_style_run(runs: &mut Vec<StyleRun>, range: Range<usize>) {
    if range.start >= range.end {
        return;
    }

    let mut out = Vec::with_capacity(runs.len() + 1);
    for run in runs.drain(..) {
        if run.end <= range.start || run.start >= range.end {
            out.push(run);
            continue;
        }
        if run.start < range.start {
            out.push(StyleRun::new(run.start, range.start, run.style_id.clone()));
        }
        if run.end > range.end {
            out.push(StyleRun::new(range.end, run.end, run.style_id));
        }
    }
    *runs = out;
}

/// Merge adjacent runs that carry the same style id.
fn merge_adjacent(runs: &mut Vec<StyleRun>) {
    let mut out: Vec<StyleRun> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let Some(last) = out.last_mut() {
            if last.end == run.start && last.style_id == run.style_id {
                last.end = run.end;
                continue;
            }
        }
        out.push(run);
    }
    *runs = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_query_main_char_style() {
        let mut alloc = StyleAlloc::new();
        alloc.set_main_char_style(2, 5, "style_char_a");

        assert_eq!(alloc.main_char_style_id_at(1), None);
        assert_eq!(alloc.main_char_style_id_at(2), Some("style_char_a"));
        assert_eq!(alloc.main_char_style_id_at(4), Some("style_char_a"));
        assert_eq!(alloc.main_char_style_id_at(5), None);
        assert_eq!(
            alloc.main_char_runs(),
            &[StyleRun::new(2, 5, "style_char_a")]
        );
    }

    #[test]
    fn adjacent_same_style_runs_merge() {
        let mut alloc = StyleAlloc::new();
        alloc.set_main_char_style(0, 3, "style_char_a");
        alloc.set_main_char_style(3, 6, "style_char_a");

        assert_eq!(
            alloc.main_char_runs(),
            &[StyleRun::new(0, 6, "style_char_a")]
        );
        assert_eq!(alloc.main_char_style_id_at(5), Some("style_char_a"));
    }

    #[test]
    fn overlapping_set_overwrites() {
        let mut alloc = StyleAlloc::new();
        alloc.set_main_char_style(0, 10, "style_char_a");
        alloc.set_main_char_style(3, 5, "style_char_b");

        assert_eq!(
            alloc.main_char_runs(),
            &[
                StyleRun::new(0, 3, "style_char_a"),
                StyleRun::new(3, 5, "style_char_b"),
                StyleRun::new(5, 10, "style_char_a"),
            ]
        );
        assert_eq!(alloc.main_char_style_id_at(3), Some("style_char_b"));
        assert_eq!(alloc.main_char_style_id_at(5), Some("style_char_a"));
    }

    #[test]
    fn clear_splits_run() {
        let mut alloc = StyleAlloc::new();
        alloc.set_main_char_style(0, 10, "style_char_a");
        alloc.clear_main_char_style(3, 5);

        assert_eq!(
            alloc.main_char_runs(),
            &[
                StyleRun::new(0, 3, "style_char_a"),
                StyleRun::new(5, 10, "style_char_a"),
            ]
        );
        assert_eq!(alloc.main_char_style_id_at(4), None);
    }

    #[test]
    fn multiple_axes_are_independent() {
        let mut alloc = StyleAlloc::new();
        alloc.set_main_char_style(0, 2, "style_char_main");
        alloc.set_ruby_char_style(0, 3, "style_char_ruby");
        alloc.set_word_ruby_style(1, 2, "style_ruby_layout");
        alloc.set_line_paragraph_style(4, 5, "style_para");
        alloc.set_page_style(0, 4, "style_page_a");
        alloc.set_page_style(4, 8, "style_page_b");

        assert_eq!(alloc.main_char_style_id_at(1), Some("style_char_main"));
        assert_eq!(alloc.ruby_char_style_id_at(2), Some("style_char_ruby"));
        assert_eq!(alloc.word_ruby_style_id_at(1), Some("style_ruby_layout"));
        assert_eq!(alloc.line_paragraph_style_id_at(4), Some("style_para"));
        assert_eq!(alloc.page_style_id_at(3), Some("style_page_a"));
        assert_eq!(alloc.page_style_id_at(4), Some("style_page_b"));
        assert_eq!(alloc.page_style_id_at(8), None);
    }

    #[test]
    fn empty_range_is_noop() {
        let mut alloc = StyleAlloc::new();
        alloc.set_main_char_style(2, 2, "style_char_a");
        assert!(alloc.main_char_runs().is_empty());
    }

    #[test]
    fn style_run_helpers() {
        let run = StyleRun::new(2, 5, "style_char_a");
        assert!(run.contains(2));
        assert!(run.contains(4));
        assert!(!run.contains(5));
        assert_eq!(run.len(), 3);
        assert!(!run.is_empty());
    }
}
