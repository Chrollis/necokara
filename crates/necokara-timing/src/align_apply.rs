//! Applying alignment results (stable-ts tokens) back onto a [`Lyrics`].
//!
//! Two entry points mirror the two text forms used for alignment:
//!  - [`align_apply_main_text`]: tokens were produced from the plain main
//!    text; every word writes its main-stream cells.
//!  - [`align_apply_reading_text`]: tokens were produced from the reading
//!    text; words with ruby write ruby cells, others write main cells, and
//!    ruby words get their main cells interpolated from the ruby span.
//!
//! Tokens are consumed strictly in order, one per written cell; a character
//! mismatch or a leftover/missing token is an error. Checks are never
//! touched. Durations are backfilled from the next cell's start; the final
//! main cell defaults to 500 ms.

use necokara_error::CkError;
use necokara_lyrics::ck_time::CkTime;
use necokara_lyrics::lyrics::Lyrics;

use crate::separate_align::AlignChar;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// An aligned token's char did not match the lyric cell's char.
    pub const CHAR_MISMATCH: &str = ck_code!("necokara-timing", align_apply, char_mismatch);
    /// Tokens ran out before all lyric cells were consumed.
    pub const TOKENS_EXHAUSTED: &str =
        ck_code!("necokara-timing", align_apply, tokens_exhausted);
    /// More tokens remained after all lyric cells were consumed.
    pub const EXTRA_TOKENS: &str = ck_code!("necokara-timing", align_apply, extra_tokens);
}

/// The default duration (ms) given to the final cell of the main stream.
const FINAL_DURATION_MS: u32 = 500;

/// A word's pre-fetched layout: its ruby/main cell ranges in the streams.
struct WordLayout {
    ruby_len: usize,
    main_range: std::ops::Range<usize>,
    ruby_range: std::ops::Range<usize>,
}

/// Read the word layout table (copying lengths/ranges) so we can mutate the
/// streams without fighting the allocator borrows.
fn layouts(lyrics: &Lyrics) -> Vec<WordLayout> {
    let mut alloc = lyrics.word_allocator.clone();
    let mut out = Vec::new();
    for wi in 0..alloc.word_count() {
        let ruby_len = alloc.words()[wi].ruby_len;
        let main_range = alloc.main_range(wi);
        let ruby_range = alloc.ruby_range(wi);
        out.push(WordLayout {
            ruby_len,
            main_range,
            ruby_range,
        });
    }
    out
}

/// Apply `tokens` (from aligning the *reading* text) back onto `lyrics`.
pub fn align_apply_reading_text(
    lyrics: &Lyrics,
    tokens: &[AlignChar],
) -> Result<Lyrics, CkError> {
    let mut lyrics = lyrics.clone();
    let lays = layouts(&lyrics);
    let mut i = 0usize;

    // Track the previously written cell (stream + index) to backfill its
    // duration once the current cell's start is known.
    let mut prev: Option<(bool, usize)> = None; // (is_ruby, cell index)

    macro_rules! write_cell {
        ($cells:expr, $is_ruby:expr, $idx:expr) => {{
            let tok = tokens.get(i).ok_or_else(|| {
                CkError::new(codes::TOKENS_EXHAUSTED, "tokens exhausted before lyrics cells")
            })?;
            let cell = &mut $cells[$idx];
            if tok.ch != cell.ch {
                return Err(CkError::new(
                    codes::CHAR_MISMATCH,
                    format!(
                        "char mismatch at token {i}: align '{0}' != lyric '{1}'",
                        tok.ch, cell.ch
                    ),
                ));
            }
            let start_ms = (tok.time * 1000.0).round() as i64;
            cell.start = Some(CkTime::new(start_ms));
            // Backfill the previous written cell's duration.
            if let Some((p_is_ruby, p_idx)) = prev {
                let (p_start, p_cell) = if p_is_ruby {
                    let c = &mut lyrics.ruby_stream.cells[p_idx];
                    (c.start.map(|t| t.ms), c)
                } else {
                    let c = &mut lyrics.main_stream.cells[p_idx];
                    (c.start.map(|t| t.ms), c)
                };
                if let Some(ps) = p_start {
                    p_cell.duration = (start_ms - ps) as u32;
                }
            }
            prev = Some(($is_ruby, $idx));
            i += 1;
        }};
    }

    for lay in &lays {
        if lay.ruby_len > 0 {
            for idx in lay.ruby_range.clone() {
                write_cell!(lyrics.ruby_stream.cells, true, idx);
            }
        } else {
            for idx in lay.main_range.clone() {
                write_cell!(lyrics.main_stream.cells, false, idx);
            }
        }
    }

    if i != tokens.len() {
        return Err(CkError::new(
            codes::EXTRA_TOKENS,
            format!(
                "{} extra alignment tokens after consuming all lyrics cells",
                tokens.len() - i
            ),
        ));
    }

    // Default duration for the final main cell.
    if let Some(last) = lyrics.main_stream.cells.last_mut() {
        last.duration = FINAL_DURATION_MS;
    }

    // Phase 2: for words with ruby, interpolate their main cells from the
    // ruby span (reading text did not write main cells for these words).
    for lay in &lays {
        if lay.ruby_len == 0 {
            continue;
        }
        let rubies: Vec<(i64, u32)> = lyrics.ruby_stream.cells[lay.ruby_range.clone()]
            .iter()
            .map(|c| (c.start.map(|t| t.ms).unwrap_or(0), c.duration))
            .collect();
        if rubies.is_empty() {
            continue;
        }
        let start_ms = rubies[0].0;
        let total_dur: u64 = rubies.iter().map(|(_, d)| *d as u64).sum();
        let main_len = lay.main_range.len();
        if main_len == 0 {
            continue;
        }
        let dt_ms = total_dur as f64 / main_len as f64;
        for (p, idx) in lay.main_range.clone().enumerate() {
            let cell = &mut lyrics.main_stream.cells[idx];
            let ms = start_ms + (dt_ms * p as f64).round() as i64;
            cell.start = Some(CkTime::new(ms));
            cell.duration = dt_ms.round() as u32;
        }
    }

    Ok(lyrics)
}

/// Apply `tokens` (from aligning the *plain main* text) back onto `lyrics`.
///
/// All words write their main cells; afterwards ruby words get their ruby
/// cells interpolated from the main span.
pub fn align_apply_main_text(
    lyrics: &Lyrics,
    tokens: &[AlignChar],
) -> Result<Lyrics, CkError> {
    let mut lyrics = lyrics.clone();
    let lays = layouts(&lyrics);
    let mut i = 0usize;
    let mut prev: Option<usize> = None; // main cell index

    for lay in &lays {
        for idx in lay.main_range.clone() {
            let tok = tokens.get(i).ok_or_else(|| {
                CkError::new(codes::TOKENS_EXHAUSTED, "tokens exhausted before lyrics cells")
            })?;
            let cell = &mut lyrics.main_stream.cells[idx];
            if tok.ch != cell.ch {
                return Err(CkError::new(
                    codes::CHAR_MISMATCH,
                    format!(
                        "char mismatch at token {i}: align '{0}' != lyric '{1}'",
                        tok.ch, cell.ch
                    ),
                ));
            }
            let start_ms = (tok.time * 1000.0).round() as i64;
            cell.start = Some(CkTime::new(start_ms));
            if let Some(p_idx) = prev {
                let p_start = lyrics.main_stream.cells[p_idx]
                    .start
                    .map(|t| t.ms)
                    .unwrap_or(0);
                lyrics.main_stream.cells[p_idx].duration = (start_ms - p_start) as u32;
            }
            prev = Some(idx);
            i += 1;
        }
    }

    if i != tokens.len() {
        return Err(CkError::new(
            codes::EXTRA_TOKENS,
            format!(
                "{} extra alignment tokens after consuming all lyrics cells",
                tokens.len() - i
            ),
        ));
    }

    if let Some(last) = lyrics.main_stream.cells.last_mut() {
        last.duration = FINAL_DURATION_MS;
    }

    // Interpolate ruby for words with ruby from their main span.
    for lay in &lays {
        if lay.ruby_len == 0 {
            continue;
        }
        let mains: Vec<i64> = lyrics.main_stream.cells[lay.main_range.clone()]
            .iter()
            .map(|c| c.start.map(|t| t.ms).unwrap_or(0))
            .collect();
        if mains.is_empty() {
            continue;
        }
        let start_ms = mains[0];
        let end_ms = if lay.main_range.end < lyrics.main_stream.cells.len() {
            lyrics.main_stream.cells[lay.main_range.end]
                .start
                .map(|t| t.ms)
                .unwrap_or(start_ms)
        } else {
            start_ms
        };
        let span = end_ms - start_ms;
        let ruby_count = lay.ruby_len;
        if ruby_count == 0 {
            continue;
        }
        let dt_ms = span as f64 / ruby_count as f64;
        for (p, idx) in lay.ruby_range.clone().enumerate() {
            let cell = &mut lyrics.ruby_stream.cells[idx];
            let ms = start_ms + (dt_ms * p as f64).round() as i64;
            cell.start = Some(CkTime::new(ms));
            cell.duration = dt_ms.round() as u32;
        }
    }

    Ok(lyrics)
}
