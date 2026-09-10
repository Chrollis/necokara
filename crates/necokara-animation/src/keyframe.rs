//! Sparse keyframe tracks and the per-line animation output.
//!
//! Python returns one [`LineAnimation`] per line. Line / main / ruby are three
//! parallel levels; ruby ownership is resolved by the renderer through
//! `WordAlloc`, so the animation data never repeats the text or the ownership.

use necokara_error::CkError;

use crate::decor::DecorItem;
use crate::easing::Easing;
use crate::state::{AnimatedLine, AnimatedMain, AnimatedRuby, AnimatedState};

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// A line animation carries no line-level keyframes, so it has no window.
    pub const LINE_KEYFRAMES_EMPTY: &str =
        ck_code!("necokara-animation", line_animation, line_keyframes_empty);
    /// The script name is empty.
    pub const SCRIPT_EMPTY: &str = ck_code!("necokara-animation", line_animation, script_empty);
    /// Keyframe times are not strictly ascending.
    pub const KEYFRAMES_UNSORTED: &str =
        ck_code!("necokara-animation", line_animation, keyframes_unsorted);
    /// A main/ruby index appears more than once in one line animation.
    pub const DUPLICATE_INDEX: &str =
        ck_code!("necokara-animation", line_animation, duplicate_index);
}

/// One sparse keyframe for a state track.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateKeyframe {
    /// Absolute timeline time in ms.
    pub time_ms: i64,
    /// State at `time_ms`.
    pub state: AnimatedState,
    /// Easing used between this keyframe and the next; ignored on the last.
    pub easing: Easing,
}

/// Keyframes for one full-stream main character.
#[derive(Debug, Clone, PartialEq)]
pub struct MainKeyframes {
    /// Full main character-stream index.
    pub char_index: usize,
    /// Sparse keyframes, ascending by time.
    pub keyframes: Vec<StateKeyframe>,
}

/// Keyframes for one full-stream ruby character.
#[derive(Debug, Clone, PartialEq)]
pub struct RubyKeyframes {
    /// Full ruby character-stream index.
    pub ruby_index: usize,
    /// Sparse keyframes, ascending by time.
    pub keyframes: Vec<StateKeyframe>,
}

/// Python output for one line.
///
/// `lead_in_ms` / `lead_out_ms` are script *inputs* and are therefore not
/// echoed here; the script only decides `animation_in_ms` / `animation_out_ms`.
/// The window is defined by the line-level keyframes.
#[derive(Debug, Clone, PartialEq)]
pub struct LineAnimation {
    /// Line index (see `necokara_lyrics::Lyrics::line_ranges`).
    pub line_index: usize,
    /// Script name that produced this animation: the script file name without
    /// `.py`, e.g. `FadeIn`. Matches [`crate::AnimationScript::script_name`].
    pub script: String,
    /// Script-chosen in-animation duration in ms.
    pub animation_in_ms: i64,
    /// Script-chosen out-animation duration in ms.
    pub animation_out_ms: i64,
    /// Line-level track spanning the animation window. Times are absolute on
    /// the material timeline; the first keyframe is the window start and the
    /// last is the window end.
    pub line: Vec<StateKeyframe>,
    /// Sparse per-main-char tracks (only animated characters).
    pub main: Vec<MainKeyframes>,
    /// Sparse per-ruby-char tracks (only animated ruby characters).
    pub ruby: Vec<RubyKeyframes>,
    /// Decor instructions.
    pub decors: Vec<DecorItem>,
}

impl LineAnimation {
    /// Build a line animation from its core parts: the line-level track defines
    /// the animation window, and `animation_in_ms` / `animation_out_ms` are the
    /// script-chosen durations. Attach main / ruby / decor data with the
    /// `with_*` builder methods.
    pub fn new(
        line_index: usize,
        script: impl Into<String>,
        animation_in_ms: i64,
        animation_out_ms: i64,
        line: Vec<StateKeyframe>,
    ) -> Self {
        Self {
            line_index,
            script: script.into(),
            animation_in_ms,
            animation_out_ms,
            line,
            main: Vec::new(),
            ruby: Vec::new(),
            decors: Vec::new(),
        }
    }

    /// Attach the sparse per-main-char tracks.
    pub fn with_main(mut self, main: Vec<MainKeyframes>) -> Self {
        self.main = main;
        self
    }

    /// Attach the sparse per-ruby-char tracks.
    pub fn with_ruby(mut self, ruby: Vec<RubyKeyframes>) -> Self {
        self.ruby = ruby;
        self
    }

    /// Attach the decor instructions.
    pub fn with_decors(mut self, decors: Vec<DecorItem>) -> Self {
        self.decors = decors;
        self
    }

    /// The animation window `(start_ms, end_ms)` derived from the line track.
    pub fn line_window(&self) -> Option<(i64, i64)> {
        let first = self.line.first()?;
        let last = self.line.last()?;
        Some((first.time_ms, last.time_ms))
    }

    /// Evaluate the line animation at `time_ms`.
    ///
    /// Returns `None` when the time is outside the line window (the line is not
    /// drawn). Main / ruby indices without a track, or whose own track does not
    /// cover `time_ms`, inherit the line state.
    pub fn sample(&self, time_ms: i64) -> Option<AnimatedLine> {
        let line_state = sample_state(&self.line, time_ms)?;

        let main = self
            .main
            .iter()
            .map(|track| AnimatedMain {
                char_index: track.char_index,
                state: sample_state(&track.keyframes, time_ms).unwrap_or(line_state),
            })
            .collect();

        let ruby = self
            .ruby
            .iter()
            .map(|track| AnimatedRuby {
                ruby_index: track.ruby_index,
                state: sample_state(&track.keyframes, time_ms).unwrap_or(line_state),
            })
            .collect();

        Some(AnimatedLine {
            line_index: self.line_index,
            state: line_state,
            main,
            ruby,
        })
    }

    /// Validate the structural invariants:
    /// - the script name is non-empty;
    /// - the line track is non-empty (it defines the window);
    /// - every track's keyframes are strictly ascending by time;
    /// - main / ruby indices are unique within this line animation.
    pub fn validate(&self) -> Result<(), CkError> {
        if self.script.is_empty() {
            return Err(CkError::new(
                codes::SCRIPT_EMPTY,
                format!(
                    "line {} has an empty animation script name",
                    self.line_index
                ),
            ));
        }
        if self.line.is_empty() {
            return Err(CkError::new(
                codes::LINE_KEYFRAMES_EMPTY,
                format!("line {} has no line-level keyframes", self.line_index),
            ));
        }
        check_sorted(&self.line, "line")?;

        let mut seen_main: Vec<usize> = Vec::with_capacity(self.main.len());
        for track in &self.main {
            if seen_main.contains(&track.char_index) {
                return Err(CkError::new(
                    codes::DUPLICATE_INDEX,
                    format!(
                        "line {} has duplicate main char_index {}",
                        self.line_index, track.char_index
                    ),
                ));
            }
            seen_main.push(track.char_index);
            check_sorted(&track.keyframes, "main")?;
        }

        let mut seen_ruby: Vec<usize> = Vec::with_capacity(self.ruby.len());
        for track in &self.ruby {
            if seen_ruby.contains(&track.ruby_index) {
                return Err(CkError::new(
                    codes::DUPLICATE_INDEX,
                    format!(
                        "line {} has duplicate ruby_index {}",
                        self.line_index, track.ruby_index
                    ),
                ));
            }
            seen_ruby.push(track.ruby_index);
            check_sorted(&track.keyframes, "ruby")?;
        }

        Ok(())
    }
}

/// Ensure a track's keyframe times are strictly ascending.
fn check_sorted(keyframes: &[StateKeyframe], label: &str) -> Result<(), CkError> {
    for window in keyframes.windows(2) {
        if window[1].time_ms <= window[0].time_ms {
            return Err(CkError::new(
                codes::KEYFRAMES_UNSORTED,
                format!(
                    "{label} keyframes are not strictly ascending: {} then {}",
                    window[0].time_ms, window[1].time_ms
                ),
            ));
        }
    }
    Ok(())
}

/// Evaluate a sparse state track at `time_ms`.
///
/// Returns `None` for an empty track or a time outside `[first, last]`. Inside
/// the range the state is interpolated using the easing stored on the left
/// keyframe of the segment.
pub fn sample_state(keyframes: &[StateKeyframe], time_ms: i64) -> Option<AnimatedState> {
    let first = keyframes.first()?;
    let last = keyframes.last()?;
    if time_ms < first.time_ms || time_ms > last.time_ms {
        return None;
    }
    if time_ms == last.time_ms {
        return Some(last.state);
    }

    // `i` is the index of the last keyframe at or before `time_ms`.
    let i = keyframes.partition_point(|k| k.time_ms <= time_ms) - 1;
    let a = &keyframes[i];
    let b = &keyframes[i + 1];
    let span = (b.time_ms - a.time_ms) as f64;
    let t = if span <= 0.0 {
        1.0
    } else {
        (time_ms - a.time_ms) as f64 / span
    };
    let eased = a.easing.apply(t);
    Some(AnimatedState::lerp(&a.state, &b.state, eased))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decor::{DecorItem, DecorTarget};
    use crate::transform::Transform3D;

    fn state(opacity: f64) -> AnimatedState {
        AnimatedState {
            visible: true,
            opacity,
            transform: Transform3D::default(),
        }
    }

    fn keyframe(time_ms: i64, opacity: f64, easing: Easing) -> StateKeyframe {
        StateKeyframe {
            time_ms,
            state: state(opacity),
            easing,
        }
    }

    #[test]
    fn sample_empty_track_is_none() {
        assert_eq!(sample_state(&[], 0), None);
    }

    #[test]
    fn sample_outside_range_is_none() {
        let track = vec![keyframe(100, 0.0, Easing::Linear), keyframe(200, 1.0, Easing::Linear)];
        assert_eq!(sample_state(&track, 99), None);
        assert_eq!(sample_state(&track, 201), None);
    }

    #[test]
    fn sample_endpoints_and_midpoint() {
        let track = vec![keyframe(100, 0.0, Easing::Linear), keyframe(200, 1.0, Easing::Linear)];
        assert_eq!(sample_state(&track, 100).unwrap().opacity, 0.0);
        assert_eq!(sample_state(&track, 200).unwrap().opacity, 1.0);
        assert_eq!(sample_state(&track, 150).unwrap().opacity, 0.5);
    }

    #[test]
    fn sample_uses_segment_easing() {
        let track = vec![keyframe(0, 0.0, Easing::QuadraticIn), keyframe(100, 1.0, Easing::Linear)];
        // QuadraticIn at t=0.5 -> 0.25.
        assert!((sample_state(&track, 50).unwrap().opacity - 0.25).abs() < 1e-9);
    }

    #[test]
    fn sample_single_keyframe_holds() {
        let track = vec![keyframe(50, 0.4, Easing::Linear)];
        assert_eq!(sample_state(&track, 50).unwrap().opacity, 0.4);
        assert_eq!(sample_state(&track, 49), None);
    }

    fn line_animation() -> LineAnimation {
        LineAnimation::new(
            0,
            "FadeIn",
            100,
            100,
            vec![
                keyframe(1000, 0.0, Easing::Linear),
                keyframe(1100, 1.0, Easing::Linear),
            ],
        )
        .with_main(vec![MainKeyframes {
            char_index: 0,
            keyframes: vec![
                keyframe(1000, 0.0, Easing::Linear),
                keyframe(1050, 1.0, Easing::Linear),
            ],
        }])
    }

    #[test]
    fn script_name_is_carried_through() {
        assert_eq!(line_animation().script, "FadeIn");
    }

    #[test]
    fn line_window_from_track() {
        assert_eq!(line_animation().line_window(), Some((1000, 1100)));
    }

    #[test]
    fn sample_line_inherits_line_state_outside_char_track() {
        let anim = line_animation();
        // At 1090 the char track is over; the char inherits the line state.
        let sampled = anim.sample(1090).unwrap();
        let line_state = sampled.state;
        assert_eq!(sampled.main.len(), 1);
        assert_eq!(sampled.main[0].char_index, 0);
        assert_eq!(sampled.main[0].state, line_state);
        assert!((line_state.opacity - 0.9).abs() < 1e-9);
    }

    #[test]
    fn sample_line_uses_char_track_inside_its_window() {
        let anim = line_animation();
        let sampled = anim.sample(1025).unwrap();
        assert!((sampled.main[0].state.opacity - 0.5).abs() < 1e-9);
    }

    #[test]
    fn sample_outside_window_is_none() {
        let anim = line_animation();
        assert!(anim.sample(999).is_none());
        assert!(anim.sample(1101).is_none());
    }

    #[test]
    fn validate_accepts_well_formed() {
        assert!(line_animation().validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_script_name() {
        let mut anim = line_animation();
        anim.script.clear();
        let err = anim.validate().unwrap_err();
        assert_eq!(err.code, codes::SCRIPT_EMPTY);
    }

    #[test]
    fn validate_rejects_empty_line_track() {
        let mut anim = line_animation();
        anim.line.clear();
        let err = anim.validate().unwrap_err();
        assert_eq!(err.code, codes::LINE_KEYFRAMES_EMPTY);
    }

    #[test]
    fn validate_rejects_unsorted_keyframes() {
        let mut anim = line_animation();
        anim.main[0].keyframes.swap(0, 1);
        let err = anim.validate().unwrap_err();
        assert_eq!(err.code, codes::KEYFRAMES_UNSORTED);
    }

    #[test]
    fn validate_rejects_duplicate_main_index() {
        let mut anim = line_animation();
        let duplicated = anim.main[0].clone();
        anim.main.push(duplicated);
        let err = anim.validate().unwrap_err();
        assert_eq!(err.code, codes::DUPLICATE_INDEX);
    }

    #[test]
    fn decors_are_kept_verbatim() {
        let anim = line_animation()
            .with_decors(vec![DecorItem::new(7, DecorTarget::Line { line_index: 0 })]);
        assert_eq!(anim.decors[0].decor_id, 7);
    }
}
