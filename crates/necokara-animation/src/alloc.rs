//! Line animation allocation: which line runs which animation script, plus the
//! animation-window extension for that line.
//!
//! The script set is dynamic: it is defined by `manifest.json` in the presets
//! folder (see [`crate::script_manifest`]), so a run stores the script's
//! **name** (its file name without the `.py` extension, e.g. `FadeIn`) rather
//! than a hard-coded enum. Adding or removing a script only touches the
//! manifest and the script file.

/// One sorted, non-overlapping line animation run over line indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAnimRun {
    /// First covered line index (inclusive).
    pub start: usize,
    /// End line index (exclusive).
    pub end: usize,
    /// Script name: the script file name without `.py`, e.g. `FadeIn`.
    ///
    /// This matches [`crate::AnimationScript::script_name`].
    pub script: String,
    /// Animation-window extension before the line's singing start.
    pub lead_in_ms: i64,
    /// Animation-window extension after the line's singing end.
    pub lead_out_ms: i64,
}

impl LineAnimRun {
    /// Build a run.
    pub fn new(
        start: usize,
        end: usize,
        script: impl Into<String>,
        lead_in_ms: i64,
        lead_out_ms: i64,
    ) -> Self {
        Self {
            start,
            end,
            script: script.into(),
            lead_in_ms,
            lead_out_ms,
        }
    }

    /// Whether the run covers `line_index`.
    pub fn contains(&self, line_index: usize) -> bool {
        self.start <= line_index && line_index < self.end
    }

    /// Whether the run is empty.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// Line animation allocation.
///
/// Runs are kept sorted by `start` and non-overlapping; setting a range
/// overwrites overlapping applications. Adjacent runs merge only when the
/// script *and* both window durations are equal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineAnimAlloc {
    runs: Vec<LineAnimRun>,
}

impl LineAnimAlloc {
    /// Build an empty allocation.
    pub fn new() -> Self {
        Self::default()
    }

    /// All runs, sorted and non-overlapping.
    pub fn runs(&self) -> &[LineAnimRun] {
        &self.runs
    }

    /// The run covering `line_index`, if any.
    pub fn run_at(&self, line_index: usize) -> Option<&LineAnimRun> {
        let pos = self.runs.partition_point(|run| run.start <= line_index);
        if pos == 0 {
            return None;
        }
        let run = &self.runs[pos - 1];
        if run.contains(line_index) {
            Some(run)
        } else {
            None
        }
    }

    /// Script name at `line_index`, or `None` when the line has no animation.
    pub fn script_at(&self, line_index: usize) -> Option<&str> {
        self.run_at(line_index).map(|run| run.script.as_str())
    }

    /// `(lead_in_ms, lead_out_ms)` at `line_index`, when animated.
    pub fn window_at(&self, line_index: usize) -> Option<(i64, i64)> {
        self.run_at(line_index)
            .map(|run| (run.lead_in_ms, run.lead_out_ms))
    }

    /// Apply a script and window to `[start, end)`, overwriting overlaps.
    ///
    /// Adjacent runs merge when the script and both window durations match.
    pub fn set(
        &mut self,
        start: usize,
        end: usize,
        script: impl Into<String>,
        lead_in_ms: i64,
        lead_out_ms: i64,
    ) {
        if start >= end {
            return;
        }

        let script = script.into();
        clear_run(&mut self.runs, start, end);

        let pos = self.runs.partition_point(|run| run.start < start);
        self.runs
            .insert(pos, LineAnimRun::new(start, end, script, lead_in_ms, lead_out_ms));
        merge_adjacent(&mut self.runs);
    }

    /// Clear applications in `[start, end)`, splitting straddling runs.
    pub fn clear(&mut self, start: usize, end: usize) {
        clear_run(&mut self.runs, start, end);
    }
}

/// Remove `[start, end)` from the run list, splitting straddling runs.
fn clear_run(runs: &mut Vec<LineAnimRun>, start: usize, end: usize) {
    if start >= end {
        return;
    }

    let mut out = Vec::with_capacity(runs.len() + 1);
    for run in runs.drain(..) {
        if run.end <= start || run.start >= end {
            out.push(run);
            continue;
        }
        if run.start < start {
            out.push(LineAnimRun::new(
                run.start,
                start,
                run.script.clone(),
                run.lead_in_ms,
                run.lead_out_ms,
            ));
        }
        if run.end > end {
            out.push(LineAnimRun::new(
                end,
                run.end,
                run.script.clone(),
                run.lead_in_ms,
                run.lead_out_ms,
            ));
        }
    }
    *runs = out;
}

/// Merge adjacent runs that carry the same script and window.
fn merge_adjacent(runs: &mut Vec<LineAnimRun>) {
    let mut out: Vec<LineAnimRun> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let Some(last) = out.last_mut() {
            if last.end == run.start
                && last.script == run.script
                && last.lead_in_ms == run.lead_in_ms
                && last.lead_out_ms == run.lead_out_ms
            {
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
    fn set_and_query() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(2, 5, "FadeIn", 200, 300);

        assert_eq!(alloc.script_at(1), None);
        assert_eq!(alloc.script_at(2), Some("FadeIn"));
        assert_eq!(alloc.script_at(4), Some("FadeIn"));
        assert_eq!(alloc.script_at(5), None);
        assert_eq!(alloc.window_at(3), Some((200, 300)));
        assert!(alloc.run_at(3).is_some());
    }

    #[test]
    fn adjacent_same_script_and_window_merge() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 3, "FadeOut", 100, 100);
        alloc.set(3, 6, "FadeOut", 100, 100);

        assert_eq!(alloc.runs().len(), 1);
        assert_eq!(alloc.runs()[0], LineAnimRun::new(0, 6, "FadeOut", 100, 100));
    }

    #[test]
    fn adjacent_different_window_does_not_merge() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 3, "FadeOut", 100, 100);
        alloc.set(3, 6, "FadeOut", 200, 100);

        assert_eq!(alloc.runs().len(), 2);
    }

    #[test]
    fn adjacent_different_script_does_not_merge() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 3, "FadeIn", 100, 100);
        alloc.set(3, 6, "FadeOut", 100, 100);

        assert_eq!(alloc.runs().len(), 2);
    }

    #[test]
    fn overlapping_set_overwrites() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 10, "FadeIn", 100, 100);
        alloc.set(3, 5, "FadeOut", 0, 0);

        assert_eq!(alloc.runs().len(), 3);
        assert_eq!(alloc.script_at(3), Some("FadeOut"));
        assert_eq!(alloc.script_at(2), Some("FadeIn"));
        assert_eq!(alloc.script_at(5), Some("FadeIn"));
        assert_eq!(alloc.window_at(5), Some((100, 100)));
    }

    #[test]
    fn clear_splits_run() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 10, "FadeInOut", 50, 50);
        alloc.clear(3, 5);

        assert_eq!(alloc.runs().len(), 2);
        assert_eq!(alloc.script_at(4), None);
        assert_eq!(alloc.script_at(2), Some("FadeInOut"));
        assert_eq!(alloc.script_at(5), Some("FadeInOut"));
    }

    #[test]
    fn empty_range_is_noop() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(4, 4, "FadeIn", 0, 0);
        assert!(alloc.runs().is_empty());
    }

    #[test]
    fn script_names_are_not_hard_coded() {
        // Any name the manifest declares works; the allocation does not know
        // the script set.
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 1, "MyCustomScript", 0, 0);
        assert_eq!(alloc.script_at(0), Some("MyCustomScript"));
    }
}
