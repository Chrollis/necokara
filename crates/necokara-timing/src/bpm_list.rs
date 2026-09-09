//! BPM segments over the song timeline.
//!
//! Tempo is modelled as a set of non-overlapping constant-BPM stretches. Each
//! stretch is stored as `start -> bpm` in a [`BTreeMap`], so:
//!  - a segment's end is derived (the next segment's `start`), and
//!  - a physical sentinel segment `{ start: i64::MIN, bpm: 0.0 }` is always
//!    present, so every point in time has a well-defined tempo (`0.0` means
//!    "no beat / unknown"). Times may legitimately be negative (LRC negative
//!    time usage), hence `i64::MIN` is the sentinel start, not zero.
//!
//! A [`BTreeMap`] keeps segments ordered and gives O(log n) insert/remove/
//! lookup, which suits frequent adjustments.

use std::collections::BTreeMap;

use necokara_error::CkError;
use necokara_lyrics::ck_time::CkTime;

/// Error codes used by BPM list operations.
pub mod codes {
    use necokara_error::ck_code;

    /// Tried to remove the sentinel segment.
    pub const SENTINEL_PROTECTED: &str =
        ck_code!("necokara-timing", bpm_list, sentinel_protected);
    /// No segment exists at the requested start.
    pub const NOT_FOUND: &str = ck_code!("necokara-timing", bpm_list, not_found);
}

/// Sentinel start: earlier than any real time.
pub const SENTINEL_START: CkTime = CkTime::new(i64::MIN);
/// Tempo value meaning "no beat / unknown".
pub const BPM_NONE: f64 = 0.0;

/// Ordered set of BPM stretches (`start -> bpm`), always containing the
/// sentinel segment.
#[derive(Debug, Clone, PartialEq)]
pub struct BpmList {
    segments: BTreeMap<CkTime, f64>,
}

impl Default for BpmList {
    fn default() -> Self {
        Self::new()
    }
}

impl BpmList {
    /// New list containing only the sentinel (`bpm 0` everywhere).
    pub fn new() -> Self {
        let mut segments = BTreeMap::new();
        segments.insert(SENTINEL_START, BPM_NONE);
        Self { segments }
    }

    /// The segments (read-only), ordered by `start`.
    pub fn segments(&self) -> &BTreeMap<CkTime, f64> {
        &self.segments
    }

    /// Number of segments (including the sentinel).
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Insert (or replace) the stretch starting at `start`. A `bpm` of
    /// [`BPM_NONE`] is allowed (explicit "no beat from here").
    pub fn insert(&mut self, start: CkTime, bpm: f64) {
        self.segments.insert(start, bpm);
    }

    /// Remove the stretch starting exactly at `start`.
    ///
    /// Errors when the start is the protected sentinel or no segment exists
    /// there.
    pub fn remove(&mut self, start: CkTime) -> Result<(), CkError> {
        if start == SENTINEL_START {
            return Err(CkError::new(
                codes::SENTINEL_PROTECTED,
                "the sentinel segment cannot be removed",
            ));
        }
        if self.segments.remove(&start).is_none() {
            return Err(CkError::new(
                codes::NOT_FOUND,
                format!("no bpm segment at {}", start.ms),
            ));
        }
        Ok(())
    }

    /// The tempo active at `time`: the most recent segment starting at or
    /// before `time` (the sentinel guarantees a result).
    pub fn bpm_at(&self, time: CkTime) -> f64 {
        self.segments
            .range(..=time)
            .next_back()
            .map(|(_, bpm)| *bpm)
            .unwrap_or(BPM_NONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: i64) -> CkTime {
        CkTime::new(v)
    }

    #[test]
    fn sentinel_gives_zero_everywhere() {
        let list = BpmList::new();
        assert_eq!(list.bpm_at(ms(-999_999)), BPM_NONE);
        assert_eq!(list.bpm_at(ms(0)), BPM_NONE);
        assert_eq!(list.bpm_at(ms(10_000)), BPM_NONE);
        assert_eq!(list.len(), 1); // just the sentinel
    }

    #[test]
    fn insert_then_query_inside_and_after() {
        let mut list = BpmList::new();
        list.insert(ms(1_000), 120.0);
        list.insert(ms(5_000), 90.0);

        // Before the first real segment: sentinel (0).
        assert_eq!(list.bpm_at(ms(0)), BPM_NONE);
        // Inside each segment.
        assert_eq!(list.bpm_at(ms(1_000)), 120.0);
        assert_eq!(list.bpm_at(ms(4_999)), 120.0);
        // At the second segment start it switches.
        assert_eq!(list.bpm_at(ms(5_000)), 90.0);
        // After all segments: last one still applies.
        assert_eq!(list.bpm_at(ms(99_999)), 90.0);
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn negative_times_work_after_sentinel() {
        let mut list = BpmList::new();
        list.insert(ms(-5_000), 100.0);
        assert_eq!(list.bpm_at(ms(-5_000)), 100.0);
        assert_eq!(list.bpm_at(ms(-4_000)), 100.0);
        assert_eq!(list.bpm_at(ms(-5_001)), BPM_NONE); // before segment, sentinel
    }

    #[test]
    fn insert_same_start_replaces() {
        let mut list = BpmList::new();
        list.insert(ms(0), 120.0);
        list.insert(ms(0), 140.0);
        assert_eq!(list.bpm_at(ms(1)), 140.0);
        assert_eq!(list.len(), 2); // sentinel + one
    }

    #[test]
    fn remove_deletes_and_sentinel_protected() {
        let mut list = BpmList::new();
        list.insert(ms(0), 120.0);
        assert!(list.remove(ms(0)).is_ok());
        assert_eq!(list.bpm_at(ms(1)), BPM_NONE);
        // Sentinel cannot be removed.
        assert!(list.remove(SENTINEL_START).is_err());
        assert_eq!(list.len(), 1);
    }
}
