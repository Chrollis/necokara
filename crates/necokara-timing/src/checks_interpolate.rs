//! Check-driven time interpolation over the character streams.
//!
//! Given the word whose first check (`checks[0]`) is the newest time anchor,
//! fill concrete `start`/`duration` into the main and ruby cells that lie
//! between the previous anchored word and this one.
//!
//! Model notes:
//!  - A word's first check is its "main time tag" (RL's `t_jikan`). Only
//!    `checks[0]` drives the cross-word interpolation.
//!  - Main cells between the two anchors are divided evenly.
//!  - For each intermediate word that has ruby: ruby is anchored at the word's
//!    own start; when `ruby_seg.len == checks.len`, timed extra checks pin
//!    ruby segment starts; remaining unset ruby is interpolated evenly inside
//!    that word's own window `[word_start, word_start + dt * main_len)`.
//!  - The target word (the `C` anchor) is not filled by this function.
//!
//! Failures (target check untimed, or no earlier anchored check) return
//! `false`; structured error codes come later.

use necokara_error::CkError;
use necokara_lyrics::allocator::WordAlloc;
use necokara_lyrics::ck_time::CkTime;
use necokara_lyrics::stream::CharStream;

/// Error codes used by check interpolation.
pub mod codes {
    use necokara_error::ck_code;

    /// The word index is out of range.
    pub const WORD_OUT_OF_RANGE: &str =
        ck_code!("necokara-timing", checks_interpolate, word_out_of_range);
    /// The target word's `checks[0]` (first direction) has no time.
    pub const FIRST_CHECK_UNTIMED: &str =
        ck_code!("necokara-timing", checks_interpolate, first_check_untimed);
    /// The target word's `checks.last` (last direction) has no time.
    pub const LAST_CHECK_UNTIMED: &str =
        ck_code!("necokara-timing", checks_interpolate, last_check_untimed);
    /// No earlier word with a timed `checks[0]` exists (first direction).
    pub const NO_PREV_ANCHOR: &str =
        ck_code!("necokara-timing", checks_interpolate, no_prev_anchor);
    /// No later word with a timed `checks.last` exists (last direction).
    pub const NO_NEXT_ANCHOR: &str =
        ck_code!("necokara-timing", checks_interpolate, no_next_anchor);
}

/// Interpolate times ending at `word_index`'s first check.
///
/// `word_index.checks[0]` must have a time (`C`). Walks back to the nearest
/// earlier word whose `checks[0]` has a time (`D`), then fills all main cells
/// from D's word start up to (not including) C's word with even spacing, and
/// interpolates each intermediate word's ruby inside that word's own window.
///
/// Errors when `word_index` is out of range, `C` is untimed, or no earlier
/// anchored `D` exists.
pub fn interpolate_from_word_first(
    alloc: &mut WordAlloc,
    word_index: usize,
    main_stream: &mut CharStream,
    ruby_stream: &mut CharStream,
) -> Result<(), CkError> {
    if word_index >= alloc.word_count() {
        return Err(CkError::new(
            codes::WORD_OUT_OF_RANGE,
            format!("word index {word_index} out of range"),
        ));
    }

    let c_time = match alloc.words()[word_index].checks.first() {
        Some(Some(t)) => *t,
        _ => {
            return Err(CkError::new(
                codes::FIRST_CHECK_UNTIMED,
                format!("word {word_index} first check has no time"),
            ))
        }
    };

    let d_index = match find_prev_anchored(alloc, word_index) {
        Some(i) => i,
        None => {
            return Err(CkError::new(
                codes::NO_PREV_ANCHOR,
                format!("no earlier anchored check before word {word_index}"),
            ))
        }
    };
    let d_time = alloc.words()[d_index].checks[0].unwrap();

    // Main cells from D's word start (inclusive) to C's word start (exclusive):
    // the C word itself is not filled.
    let main_begin = alloc.main_range(d_index).start;
    let main_end = alloc.main_range(word_index).start;
    if main_end <= main_begin {
        return Ok(()); // adjacent words, nothing in between
    }
    let main_count = main_end - main_begin;
    let span_ms = c_time.ms - d_time.ms;
    let dt = span_ms as f64 / main_count as f64;

    // Fill main cells evenly.
    for (i, cell) in main_stream.cells[main_begin..main_end].iter_mut().enumerate() {
        let ms = d_time.ms + (i as f64 * dt).round() as i64;
        cell.start = Some(CkTime::new(ms));
        cell.duration = dt.round() as u32;
    }

    // Per-word ruby interpolation over the intermediate words.
    let mut cursor = main_begin; // main-stream index of the current word start
    for wi in d_index..word_index {
        let main_len = alloc.words()[wi].main_len;
        let word_start_ms = d_time.ms + ((cursor - main_begin) as f64 * dt).round() as i64;
        let word_end_ms = word_start_ms + (dt * main_len as f64).round() as i64;

        interpolate_word_ruby(
            alloc,
            wi,
            ruby_stream,
            CkTime::new(word_start_ms),
            CkTime::new(word_end_ms),
        );

        cursor += main_len;
    }

    Ok(())
}

/// Interpolate times starting from `word_index`'s last check, walking
/// *forward* (toward larger word indexes).
///
/// Symmetric to [`interpolate_from_word_first`]: `word_index.checks.last`
/// must have a time (`C`). Walks forward to the nearest later word whose
/// `checks.last` has a time (`D'`), then fills all main cells from the word
/// after `word_index` up to (including) D''s word with even spacing over
/// `(C, D']`.
///
/// The triggering word itself is never filled (mirroring the first-check
/// rule where the C word is not filled).
pub fn interpolate_from_word_last(
    alloc: &mut WordAlloc,
    word_index: usize,
    main_stream: &mut CharStream,
    ruby_stream: &mut CharStream,
) -> Result<(), CkError> {
    if word_index >= alloc.word_count() {
        return Err(CkError::new(
            codes::WORD_OUT_OF_RANGE,
            format!("word index {word_index} out of range"),
        ));
    }

    let c_time = match alloc.words()[word_index].checks.last() {
        Some(Some(t)) => *t,
        _ => {
            return Err(CkError::new(
                codes::LAST_CHECK_UNTIMED,
                format!("word {word_index} last check has no time"),
            ))
        }
    };

    // Walk forward to the next word with a timed checks.last (D').
    let d_index = match find_next_anchored_last(alloc, word_index) {
        Some(i) => i,
        None => {
            return Err(CkError::new(
                codes::NO_NEXT_ANCHOR,
                format!("no later anchored check after word {word_index}"),
            ))
        }
    };
    let d_time = alloc.words()[d_index].checks.last().unwrap().unwrap();

    // Fill main cells after the trigger word, up to and including D''s word.
    let main_begin = alloc.main_range(word_index).end; // trigger word's end (exclusive of it)
    let main_end = alloc.main_range(d_index).end; // D''s end (inclusive)
    if main_end <= main_begin {
        return Ok(()); // nothing in between
    }
    let main_count = main_end - main_begin;
    let span_ms = d_time.ms - c_time.ms;
    let dt = span_ms as f64 / main_count as f64;

    // First filled cell = C + dt; last filled cell = D' (anchor).
    for (i, cell) in main_stream.cells[main_begin..main_end].iter_mut().enumerate() {
        let ms = c_time.ms + ((i as f64 + 1.0) * dt).round() as i64;
        cell.start = Some(CkTime::new(ms));
        cell.duration = dt.round() as u32;
    }

    // Ruby interpolation is a forward pass from the first filled word to D'.
    let mut cursor = main_begin;
    for wi in (word_index + 1)..=d_index {
        let main_len = alloc.words()[wi].main_len;
        let word_start_ms = c_time.ms + ((cursor - main_begin + 1) as f64 * dt).round() as i64;
        let word_end_ms = word_start_ms + (dt * main_len as f64).round() as i64;

        interpolate_word_ruby(
            alloc,
            wi,
            ruby_stream,
            CkTime::new(word_start_ms),
            CkTime::new(word_end_ms),
        );

        cursor += main_len;
    }

    Ok(())
}

/// Both interpolation directions; fails when either direction fails.
pub fn interpolate_to_word(
    alloc: &mut WordAlloc,
    word_index: usize,
    main_stream: &mut CharStream,
    ruby_stream: &mut CharStream,
) -> Result<(), CkError> {
    interpolate_from_word_first(alloc, word_index, main_stream, ruby_stream)?;
    interpolate_from_word_last(alloc, word_index, main_stream, ruby_stream)
}

/// Nearest earlier word whose `checks[0]` has a time.
fn find_prev_anchored(alloc: &WordAlloc, word_index: usize) -> Option<usize> {
    (0..word_index)
        .rev()
        .find(|&i| matches!(alloc.words()[i].checks.first(), Some(Some(_))))
}

/// Nearest later word whose `checks.last` has a time.
fn find_next_anchored_last(alloc: &WordAlloc, word_index: usize) -> Option<usize> {
    (word_index + 1..alloc.word_count())
        .find(|&i| matches!(alloc.words()[i].checks.last(), Some(Some(_))))
}

/// Interpolate one word's ruby cells inside `[word_start, word_end)`.
///
/// Anchors: `ruby[0]` is pinned to `word_start`; if
/// `ruby_seg.len == checks.len`, each `checks[i]` with a time pins
/// `ruby[i].start`. Unset ruby segments between anchors are evenly spaced,
/// and `word_end` acts as the final anchor so ruby never exceeds the word's
/// own main duration.
fn interpolate_word_ruby(
    alloc: &mut WordAlloc,
    word_index: usize,
    ruby_stream: &mut CharStream,
    word_start: CkTime,
    word_end: CkTime,
) {
    // Copy the fields we need before taking a mutable borrow on `alloc`
    // (ruby_range needs &mut to rebuild prefix sums).
    let seg_lens: Vec<usize> = alloc.words()[word_index].ruby_seg_lens.clone();
    let checks: Vec<Option<CkTime>> = alloc.words()[word_index].checks.clone();
    let seg_count = seg_lens.len();
    if seg_count == 0 {
        return; // no ruby
    }

    let ruby_range = alloc.ruby_range(word_index);
    let seg_count_u = seg_count;

    // Known start time per ruby segment (None = to interpolate).
    let mut known: Vec<Option<i64>> = vec![None; seg_count_u];

    // ruby[0] anchored at word start.
    known[0] = Some(word_start.ms);

    // Extra checks pin ruby segment starts when counts align.
    if seg_count == checks.len() {
        for (i, check) in checks.iter().enumerate().take(seg_count).skip(1) {
            if let Some(t) = check {
                known[i] = Some(t.ms);
            }
        }
    }

    // Evenly fill gaps between consecutive known anchors; word_end is the
    // pseudo-anchor after the last segment.
    let mut prev: Option<(usize, i64)> = None;
    let mut i = 0usize;
    while i < seg_count_u {
        if let Some(t) = known[i] {
            prev = Some((i, t));
            i += 1;
            continue;
        }
        let mut j = i;
        while j < seg_count_u && known[j].is_none() {
            j += 1;
        }
        let (g0, t0) = prev.expect("segment 0 is always known");
        let t1 = if j < seg_count_u { known[j].unwrap() } else { word_end.ms };
        let intervals = (j - g0) as f64;
        if intervals > 0.0 {
            let step = (t1 - t0) as f64 / intervals;
            for k in 1..(j - g0) {
                known[g0 + k] = Some((t0 as f64 + k as f64 * step).round() as i64);
            }
        }
        i = j;
    }

    // Write starts + durations into the ruby cells.
    let cells = &mut ruby_stream.cells[ruby_range];
    let mut seg_start = 0usize;
    for seg in 0..seg_count_u {
        let seg_len = seg_lens[seg];
        let t0 = known[seg].unwrap_or(word_end.ms);
        let t1 = if seg + 1 < seg_count_u {
            known[seg + 1].unwrap_or(word_end.ms)
        } else {
            word_end.ms
        };
        if seg_len == 0 {
            continue;
        }
        let dur_ms = (t1 - t0).max(0) as f64;
        let step = dur_ms / seg_len as f64;
        for c in 0..seg_len {
            let cell = &mut cells[seg_start + c];
            cell.start = Some(CkTime::new((t0 as f64 + c as f64 * step).round() as i64));
            cell.duration = step.round() as u32;
        }
        seg_start += seg_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use necokara_lyrics::allocator::{CheckKind, WordSeg};
    use necokara_lyrics::stream::CharStream;

    fn ms(v: i64) -> CkTime {
        CkTime::new(v)
    }

    /// A word with a main cell for each char of `main`, no ruby.
    fn plain_word(main: &str, checks: Vec<Option<CkTime>>) -> WordSeg {
        WordSeg {
            main_len: main.chars().count(),
            ruby_len: 0,
            ruby_seg_lens: Vec::new(),
            check_kind: CheckKind::Down,
            checks,
        }
    }

    /// FIRST direction scenario:
    ///   word0 "a" (checks[0]=1000, the D anchor),
    ///   word1 "bc" with ruby "xy" split [1,1], no timed checks (to interpolate),
    ///   word2 "\n" (checks[0]=2000, the C trigger).
    /// Call interpolate_from_word_first(2).
    fn first_setup() -> (WordAlloc, CharStream, CharStream) {
        let alloc = WordAlloc::from_words(vec![
            plain_word("a", vec![Some(ms(1000))]),
            WordSeg {
                main_len: 2,
                ruby_len: 2,
                ruby_seg_lens: vec![1, 1],
                check_kind: CheckKind::Down,
                checks: vec![None, None],
            },
            plain_word("\n", vec![Some(ms(2000))]),
        ]);
        let main = CharStream::from_chars("abc\n");
        let ruby = CharStream::from_chars("xy");
        (alloc, main, ruby)
    }

    #[test]
    fn first_fills_main_evenly_and_leaves_trigger_word() {
        let (mut alloc, mut main, mut ruby) = first_setup();
        assert!(interpolate_from_word_first(&mut alloc, 2, &mut main, &mut ruby).is_ok());

        // Cells a(0), b(1), c(2) span 1000..2000 over 3 cells, dt=333.33.
        // a is the D word's own char and is filled from the D time.
        let cells = &main.cells;
        assert_eq!(cells[0].start, Some(ms(1000)));
        assert_eq!(cells[1].start, Some(ms(1333)));
        assert_eq!(cells[2].start, Some(ms(1667)));
        // \n (C word, index 3) untouched.
        assert_eq!(cells[3].start, None);
        assert_eq!(cells[0].duration, 333);
    }

    #[test]
    fn first_ruby_anchored_by_word_start() {
        let (mut alloc, mut main, mut ruby) = first_setup();
        assert!(interpolate_from_word_first(&mut alloc, 2, &mut main, &mut ruby).is_ok());

        // word1 starts at main cell 1 -> 1333. seg_count(2) != timed checks(0)
        // so no extra pins; both ruby cells interpolate in word1's window.
        let cells = &ruby.cells;
        assert_eq!(cells[0].start, Some(ms(1333)));
        // window end = 1333 + dt*2 = 1333+667 = 2000; two cells split evenly.
        assert_eq!(cells[1].start, Some(ms(1667)));
    }

    #[test]
    fn first_untimed_target_returns_false() {
        let alloc = WordAlloc::from_words(vec![
            plain_word("a", vec![Some(ms(1000))]),
            plain_word("\n", vec![None]),
        ]);
        let mut alloc = alloc;
        let mut main = CharStream::from_chars("a\n");
        let mut ruby = CharStream::from_chars("");
        assert!(interpolate_from_word_first(&mut alloc, 1, &mut main, &mut ruby).is_err());
    }

    #[test]
    fn first_no_earlier_anchor_returns_false() {
        let alloc = WordAlloc::from_words(vec![
            plain_word("a", vec![None]),
            plain_word("\n", vec![Some(ms(2000))]),
        ]);
        let mut alloc = alloc;
        let mut main = CharStream::from_chars("a\n");
        let mut ruby = CharStream::from_chars("");
        assert!(interpolate_from_word_first(&mut alloc, 1, &mut main, &mut ruby).is_err());
    }

    /// LAST direction scenario (mirrors the discussion example):
    ///   word0 "a" (checks.last=1000, the C trigger),
    ///   word1 "b", word2 "c", word3 "d", word4 "e\n" (checks.last=5000, D').
    /// All single-char, no ruby. Call interpolate_from_word_last(0).
    fn last_setup() -> (WordAlloc, CharStream, CharStream) {
        let alloc = WordAlloc::from_words(vec![
            plain_word("a", vec![Some(ms(1000))]),
            plain_word("b", vec![None]),
            plain_word("c", vec![None]),
            plain_word("d", vec![None]),
            plain_word("e", vec![None, Some(ms(5000))]),
        ]);
        let main = CharStream::from_chars("abcde");
        let ruby = CharStream::from_chars("");
        (alloc, main, ruby)
    }

    #[test]
    fn last_fills_after_trigger_up_to_next_anchor() {
        let (mut alloc, mut main, mut ruby) = last_setup();
        assert!(interpolate_from_word_last(&mut alloc, 0, &mut main, &mut ruby).is_ok());

        let cells = &main.cells;
        // Trigger word "a" is not filled.
        assert_eq!(cells[0].start, None);
        // b,c,d,e split (1000, 5000] over 4 cells, dt=1000:
        // b=2000, c=3000, d=4000, e=5000 (e is D''s char = anchor).
        assert_eq!(cells[1].start, Some(ms(2000)));
        assert_eq!(cells[2].start, Some(ms(3000)));
        assert_eq!(cells[3].start, Some(ms(4000)));
        assert_eq!(cells[4].start, Some(ms(5000)));
    }

    #[test]
    fn last_untimed_trigger_returns_false() {
        let alloc = WordAlloc::from_words(vec![
            plain_word("a", vec![None]),
            plain_word("e", vec![None, Some(ms(5000))]),
        ]);
        let mut alloc = alloc;
        let mut main = CharStream::from_chars("ae");
        let mut ruby = CharStream::from_chars("");
        assert!(interpolate_from_word_last(&mut alloc, 0, &mut main, &mut ruby).is_err());
    }

    #[test]
    fn last_no_later_anchor_returns_false() {
        let alloc = WordAlloc::from_words(vec![
            plain_word("a", vec![Some(ms(1000))]),
            plain_word("e", vec![None]),
        ]);
        let mut alloc = alloc;
        let mut main = CharStream::from_chars("ae");
        let mut ruby = CharStream::from_chars("");
        assert!(interpolate_from_word_last(&mut alloc, 0, &mut main, &mut ruby).is_err());
    }

    /// to_word = first && last on a word that anchors both sides.
    #[test]
    fn to_word_requires_both_directions() {
        // word1 "b" has checks[0]=1000 (D for first, as trigger C_first too?)
        // Simpler: pick word1 whose checks[0]=1000 timed but no last anchor ->
        // last fails -> to_word false.
        let alloc = WordAlloc::from_words(vec![
            plain_word("a", vec![Some(ms(500))]),
            plain_word("b", vec![Some(ms(1000))]),
            plain_word("c", vec![None]),
        ]);
        let mut alloc = alloc;
        let mut main = CharStream::from_chars("abc");
        let mut ruby = CharStream::from_chars("");
        // word1: first has D=word0(500); last has no later anchor -> false.
        assert!(interpolate_to_word(&mut alloc, 1, &mut main, &mut ruby).is_err());
    }
}
