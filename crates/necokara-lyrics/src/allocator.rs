//! Word allocation: how the flat main/ruby character streams are grouped into
//! words, plus each word's ruby segmentation and check anchors.
//!
//! Data model notes (per v0.3 discussion):
//!  - A word owns a span of main characters and a span of ruby characters
//!    (the ruby split into per-main-character segments via `ruby_seg_lens`).
//!  - Checks live at word level: `check_kind` is a single kind for the whole
//!    word (down or up), `checks` is a parallel `Vec<Option<time>>` whose
//!    length is the check count; `None` = check placed but time not yet set.
//!    How many checks a word has by default is generated elsewhere; core only
//!    stores and reads them.
//!
//! Prefix sums (`main_pref`, `ruby_pref`, `ruby_seg_prefs`) are a lazily
//! rebuilt cache: editing marks the allocator dirty, and any range query
//! rebuilds them on demand (see [`WordAlloc::ensure_built`]).

use crate::ck_time::CkTime;

/// Kind of a word's checks. One word uses a single kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CheckKind {
    /// Key down (space pressed): the ordinary singing beat.
    #[default]
    Down,
    /// Key up (space released): used for line ends / pauses.
    Up,
}

/// One word: main/ruby spans over the streams + ruby segmentation + checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordSeg {
    /// Number of main characters this word covers.
    pub main_len: usize,
    /// Total number of ruby characters this word covers.
    pub ruby_len: usize,
    /// Ruby segmentation: each entry is the ruby length attached to one main
    /// character; `sum(ruby_seg_lens) == ruby_len` must hold.
    pub ruby_seg_lens: Vec<usize>,
    /// Kind shared by all of this word's checks.
    pub check_kind: CheckKind,
    /// Check anchors: length = check count; `None` = not yet timed.
    pub checks: Vec<Option<CkTime>>,
}

impl WordSeg {
    /// A word with no ruby and no checks.
    pub fn plain(main_len: usize) -> Self {
        Self {
            main_len,
            ruby_len: 0,
            ruby_seg_lens: Vec::new(),
            check_kind: CheckKind::Down,
            checks: Vec::new(),
        }
    }

    /// A word with ruby attached, segmented per main character.
    pub fn with_ruby(main_len: usize, ruby_seg_lens: Vec<usize>) -> Self {
        let ruby_len = ruby_seg_lens.iter().sum();
        Self {
            main_len,
            ruby_len,
            ruby_seg_lens,
            check_kind: CheckKind::Down,
            checks: Vec::new(),
        }
    }

    /// Number of checks (the check count).
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Whether ruby segmentation is internally consistent.
    pub fn ok(&self) -> bool {
        self.ruby_len == self.ruby_seg_lens.iter().sum::<usize>()
    }
}

/// The word list plus lazily rebuilt prefix sums over the streams.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WordAlloc {
    words: Vec<WordSeg>,
    /// Set by editing; cleared when prefix sums are rebuilt.
    dirty: bool,
    // Prefix-sum caches (rebuilt on demand).
    main_pref: Vec<usize>,
    ruby_pref: Vec<usize>,
    ruby_seg_prefs: Vec<Vec<usize>>,
}

impl WordAlloc {
    /// New empty allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from a word list (computes prefix sums immediately).
    pub fn from_words(words: Vec<WordSeg>) -> Self {
        let mut this = Self {
            words,
            ..Self::default()
        };
        this.build();
        this
    }

    /// Rebuild prefix-sum caches from the current word list and clear dirty.
    pub fn build(&mut self) {
        self.main_pref = Vec::with_capacity(self.words.len() + 1);
        self.ruby_pref = Vec::with_capacity(self.words.len() + 1);
        self.ruby_seg_prefs = Vec::with_capacity(self.words.len());
        self.main_pref.push(0);
        self.ruby_pref.push(0);

        let mut main_sum = 0usize;
        let mut ruby_sum = 0usize;

        for word in &self.words {
            // Ruby segment boundaries are word-local offsets added to the
            // word's ruby start (`ruby_sum` at this point); they must NOT
            // mutate the global ruby sum.
            let base = ruby_sum;
            let mut seg_pref = Vec::with_capacity(word.ruby_seg_lens.len() + 1);
            seg_pref.push(base);
            let mut acc = 0usize;
            for &seg_len in &word.ruby_seg_lens {
                acc += seg_len;
                seg_pref.push(base + acc);
            }

            main_sum += word.main_len;
            ruby_sum += word.ruby_len;
            self.main_pref.push(main_sum);
            self.ruby_pref.push(ruby_sum);
            self.ruby_seg_prefs.push(seg_pref);
        }

        self.dirty = false;
    }

    /// Rebuild caches if the allocator is dirty (kept for the upcoming editing
    /// layer; reading code calls this before any range query).
    pub fn ensure_built(&mut self) {
        if self.dirty {
            self.build();
        }
    }

    /// Mark the allocator dirty (editing entry point; range queries will
    /// rebuild on next access).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Whether the prefix caches need rebuilding.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Number of words.
    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    /// Whether there are no words.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// The word list (read-only).
    pub fn words(&self) -> &[WordSeg] {
        &self.words
    }

    /// Rebuild if dirty, then the main-stream range `[start, end)` of a word.
    pub fn main_range(&mut self, word_index: usize) -> std::ops::Range<usize> {
        self.ensure_built();
        self.main_pref[word_index]..self.main_pref[word_index + 1]
    }

    /// Rebuild if dirty, then the ruby-stream range of a word.
    pub fn ruby_range(&mut self, word_index: usize) -> std::ops::Range<usize> {
        self.ensure_built();
        self.ruby_pref[word_index]..self.ruby_pref[word_index + 1]
    }

    /// Rebuild if dirty, then the ruby-stream range of one ruby segment of a
    /// word (segment `j` belongs to main character `j` of that word).
    pub fn ruby_seg_range(
        &mut self,
        word_index: usize,
        seg_index: usize,
    ) -> std::ops::Range<usize> {
        self.ensure_built();
        let prefs = &self.ruby_seg_prefs[word_index];
        prefs[seg_index]..prefs[seg_index + 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Words matching main "明日\n" (3 main: 明,日,\n) + ruby あ/し/た per char.
    /// word0 = 明日 with ruby segs [1,1] + 2 checks; word1 = \n (no ruby).
    fn two_word_alloc() -> WordAlloc {
        WordAlloc::from_words(vec![
            WordSeg {
                main_len: 2,
                ruby_len: 2,
                ruby_seg_lens: vec![1, 1],
                check_kind: CheckKind::Down,
                checks: vec![Some(CkTime::new(1000)), Some(CkTime::new(2000))],
            },
            WordSeg::plain(1), // the \n word
        ])
    }

    #[test]
    fn word_seg_ok() {
        let w = WordSeg::with_ruby(2, vec![1, 1]);
        assert!(w.ok());
        let bad = WordSeg {
            ruby_len: 3,
            ruby_seg_lens: vec![1, 1],
            ..WordSeg::plain(2)
        };
        assert!(!bad.ok());
    }

    #[test]
    fn ranges_after_build() {
        let mut a = two_word_alloc();
        // main: word0 covers [0,2), word1 [2,3).
        assert_eq!(a.main_range(0), 0..2);
        assert_eq!(a.main_range(1), 2..3);
        // ruby: word0 covers [0,2); word1 none.
        assert_eq!(a.ruby_range(0), 0..2);
        assert_eq!(a.ruby_range(1), 2..2);
        // seg 0 -> ruby [0,1), seg 1 -> [1,2).
        assert_eq!(a.ruby_seg_range(0, 0), 0..1);
        assert_eq!(a.ruby_seg_range(0, 1), 1..2);
    }

    #[test]
    fn dirty_rebuild_reflects_word_changes() {
        let mut a = two_word_alloc();
        assert!(!a.is_dirty());

        // Simulate an edit: lengthen word0 by one main char + one ruby char.
        a.words[0].main_len = 3;
        a.words[0].ruby_len = 3;
        a.words[0].ruby_seg_lens = vec![1, 1, 1];
        a.mark_dirty();
        assert!(a.is_dirty());

        // Range query rebuilds and must reflect the new lengths.
        assert_eq!(a.main_range(0), 0..3);
        assert!(!a.is_dirty());
        assert_eq!(a.main_range(1), 3..4);
        assert_eq!(a.ruby_range(0), 0..3);
        assert_eq!(a.ruby_seg_range(0, 2), 2..3);
    }

    #[test]
    fn dirty_rebuild_after_removal() {
        let mut a = two_word_alloc();
        // Remove the trailing \n word entirely.
        a.words.pop();
        a.mark_dirty();
        assert_eq!(a.main_range(0), 0..2);
        assert_eq!(a.word_count(), 1);
    }

    #[test]
    fn build_clears_dirty_and_noop_build_is_fine() {
        let mut a = two_word_alloc();
        a.mark_dirty();
        a.build();
        assert!(!a.is_dirty());
        assert_eq!(a.main_range(1), 2..3);
    }
}
