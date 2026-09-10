//! Decor: renderer-interpreted decoration instructions.
//!
//! The animation layer never rasterizes glyphs, so decor placement that depends
//! on glyph ink is expressed as symbolic [`DecorAnchor`]s. The WebGL renderer
//! resolves them against its glyph atlas / ink metrics at draw time; no ink
//! geometry crosses the IPC boundary.

use std::collections::BTreeMap;

use necokara_error::CkError;
use necokara_style::CkColor;

use crate::easing::Easing;
use crate::keyframe::sample_state;
use crate::state::AnimatedState;
use crate::transform::lerp;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// A decor's active window is inverted (`start_ms > end_ms`).
    pub const WINDOW_INVERTED: &str = ck_code!("necokara-animation", decor, window_inverted);
    /// A decor parameter track's keyframes are not strictly ascending.
    pub const PARAM_KEYFRAMES_UNSORTED: &str =
        ck_code!("necokara-animation", decor, param_keyframes_unsorted);
}

/// A positional reference used by decor parameters.
///
/// The animation layer cannot rasterize glyphs, so it emits symbolic
/// references; the WebGL renderer resolves them against its glyph atlas and
/// ink metrics at draw time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecorAnchor {
    /// Absolute canvas pixels.
    Canvas { x_px: f64, y_px: f64 },

    /// Normalized inside the target's layout box; `(0,0)` = box top-left,
    /// `(1,1)` = box bottom-right. For main / ruby targets this is the glyph
    /// advance/layout box, not the ink.
    Box { u: f64, v: f64 },

    /// Normalized inside the target glyph's ink bounding box.
    InkBox { u: f64, v: f64 },

    /// Parameter along the target glyph's ink outline, `t` in `0..=1`.
    /// Intended for stroke-following effects (particles along the contour).
    InkOutline { t: f64 },

    /// Deterministic sample inside the target glyph's ink area. `seed` keeps
    /// the sample stable frame-to-frame (particle continuity).
    InkArea { u: f64, v: f64, seed: u64 },
}

impl DecorAnchor {
    /// Interpolate two anchors of the **same variant**; returns `None` when the
    /// variants differ (the caller then steps instead).
    pub fn lerp(a: &Self, b: &Self, t: f64) -> Option<Self> {
        match (a, b) {
            (
                DecorAnchor::Canvas { x_px: ax, y_px: ay },
                DecorAnchor::Canvas { x_px: bx, y_px: by },
            ) => Some(DecorAnchor::Canvas {
                x_px: lerp(*ax, *bx, t),
                y_px: lerp(*ay, *by, t),
            }),
            (DecorAnchor::Box { u: au, v: av }, DecorAnchor::Box { u: bu, v: bv }) => {
                Some(DecorAnchor::Box {
                    u: lerp(*au, *bu, t),
                    v: lerp(*av, *bv, t),
                })
            }
            (
                DecorAnchor::InkBox { u: au, v: av },
                DecorAnchor::InkBox { u: bu, v: bv },
            ) => Some(DecorAnchor::InkBox {
                u: lerp(*au, *bu, t),
                v: lerp(*av, *bv, t),
            }),
            (DecorAnchor::InkOutline { t: at }, DecorAnchor::InkOutline { t: bt }) => {
                Some(DecorAnchor::InkOutline {
                    t: lerp(*at, *bt, t),
                })
            }
            (
                DecorAnchor::InkArea {
                    u: au,
                    v: av,
                    seed: aseed,
                },
                DecorAnchor::InkArea {
                    u: bu,
                    v: bv,
                    seed: bseed,
                },
            ) if aseed == bseed => Some(DecorAnchor::InkArea {
                u: lerp(*au, *bu, t),
                v: lerp(*av, *bv, t),
                seed: *aseed,
            }),
            _ => None,
        }
    }
}

/// A decor parameter value.
#[derive(Debug, Clone, PartialEq)]
pub enum DecorValue {
    /// A scalar number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// A string.
    Text(String),
    /// A color.
    Color(CkColor),
    /// A list of numbers.
    Numbers(Vec<f64>),
    /// A list of colors.
    Colors(Vec<CkColor>),
    /// A symbolic positional reference resolved by the renderer.
    Anchor(DecorAnchor),
    /// Several anchors (paths, multi-point effects, ...).
    Anchors(Vec<DecorAnchor>),
}

impl DecorValue {
    /// Interpolate two values.
    ///
    /// Numeric values, colors, same-variant anchors, and equal-length lists of
    /// those interpolate component-wise. Booleans, text, variant switches, and
    /// length changes step (keep `a` until the next keyframe).
    pub fn lerp(a: &Self, b: &Self, t: f64) -> Self {
        match (a, b) {
            (DecorValue::Number(x), DecorValue::Number(y)) => DecorValue::Number(lerp(*x, *y, t)),
            (DecorValue::Color(x), DecorValue::Color(y)) => {
                DecorValue::Color(lerp_color(*x, *y, t))
            }
            (DecorValue::Numbers(x), DecorValue::Numbers(y)) if x.len() == y.len() => {
                DecorValue::Numbers(x.iter().zip(y).map(|(a, b)| lerp(*a, *b, t)).collect())
            }
            (DecorValue::Colors(x), DecorValue::Colors(y)) if x.len() == y.len() => {
                DecorValue::Colors(x.iter().zip(y).map(|(a, b)| lerp_color(*a, *b, t)).collect())
            }
            (DecorValue::Anchor(x), DecorValue::Anchor(y)) => match DecorAnchor::lerp(x, y, t) {
                Some(anchor) => DecorValue::Anchor(anchor),
                None => a.clone(),
            },
            (DecorValue::Anchors(x), DecorValue::Anchors(y)) if x.len() == y.len() => {
                match x
                    .iter()
                    .zip(y)
                    .map(|(a, b)| DecorAnchor::lerp(a, b, t))
                    .collect::<Option<Vec<_>>>()
                {
                    Some(anchors) => DecorValue::Anchors(anchors),
                    None => a.clone(),
                }
            }
            _ => a.clone(),
        }
    }
}

/// Component-wise linear interpolation of an RGBA color.
fn lerp_color(a: CkColor, b: CkColor, t: f64) -> CkColor {
    fn channel(a: u8, b: u8, t: f64) -> u8 {
        (lerp(a as f64, b as f64, t)).round().clamp(0.0, 255.0) as u8
    }
    CkColor::new(
        channel(a.r, b.r, t),
        channel(a.g, b.g, t),
        channel(a.b, b.b, t),
        channel(a.a, b.a, t),
    )
}

/// A renderer-interpreted decor specification.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecorSpec {
    /// Decor type, e.g. `"particle"` or `"texture"`.
    pub kind: String,
    /// Schema-free static parameters (base values). A [`DecorParamTrack`] with
    /// the same key overrides a value over time.
    pub params: BTreeMap<String, DecorValue>,
}

/// What a decor is attached to; also selects which ink a [`DecorAnchor`]
/// resolves against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecorTarget {
    /// The whole line (ink anchors resolve against the line's combined ink).
    Line { line_index: usize },
    /// One main character (full main character-stream index).
    Main { line_index: usize, char_index: usize },
    /// One ruby character (full ruby character-stream index).
    Ruby { line_index: usize, ruby_index: usize },
}

/// One animated decor parameter keyframe.
#[derive(Debug, Clone, PartialEq)]
pub struct DecorParamKeyframe {
    /// Absolute timeline time in ms.
    pub time_ms: i64,
    /// Parameter value at `time_ms`.
    pub value: DecorValue,
    /// Easing used between this keyframe and the next; ignored on the last.
    pub easing: Easing,
}

/// Animated override for one decor parameter key.
#[derive(Debug, Clone, PartialEq)]
pub struct DecorParamTrack {
    /// Key into [`DecorSpec::params`]; the track overrides the static value.
    pub key: String,
    /// Sparse keyframes, ascending by time.
    pub keyframes: Vec<DecorParamKeyframe>,
}

impl DecorParamTrack {
    /// Build a parameter track.
    pub fn new(key: impl Into<String>, keyframes: Vec<DecorParamKeyframe>) -> Self {
        Self {
            key: key.into(),
            keyframes,
        }
    }

    /// Evaluate the parameter at `time_ms`.
    ///
    /// Returns `None` for an empty track or a time outside
    /// `[first, last]`; the caller then falls back to the static parameter.
    pub fn sample(&self, time_ms: i64) -> Option<DecorValue> {
        let first = self.keyframes.first()?;
        let last = self.keyframes.last()?;
        if time_ms < first.time_ms || time_ms > last.time_ms {
            return None;
        }
        if time_ms == last.time_ms {
            return Some(last.value.clone());
        }

        let i = self.keyframes.partition_point(|k| k.time_ms <= time_ms) - 1;
        let a = &self.keyframes[i];
        let b = &self.keyframes[i + 1];
        let span = (b.time_ms - a.time_ms) as f64;
        let t = if span <= 0.0 {
            1.0
        } else {
            (time_ms - a.time_ms) as f64 / span
        };
        let eased = a.easing.apply(t);
        Some(DecorValue::lerp(&a.value, &b.value, eased))
    }
}

/// Decor lifetime inside a line animation.
#[derive(Debug, Clone, PartialEq)]
pub struct DecorItem {
    /// Stable id within this line animation (renderer-side particle
    /// continuity).
    pub decor_id: u64,
    /// What the decor attaches to.
    pub target: DecorTarget,
    /// Active window start, absolute ms.
    pub start_ms: i64,
    /// Active window end, absolute ms.
    pub end_ms: i64,
    /// Static spec (kind + base params).
    pub spec: DecorSpec,
    /// Sparse decor state (visible / opacity / transform) over the window.
    pub keyframes: Vec<crate::keyframe::StateKeyframe>,
    /// Per-key animated parameter overrides.
    pub param_keyframes: Vec<DecorParamTrack>,
}

impl DecorItem {
    /// Build a decor with an empty spec, zero-length window, and no tracks.
    pub fn new(decor_id: u64, target: DecorTarget) -> Self {
        Self {
            decor_id,
            target,
            start_ms: 0,
            end_ms: 0,
            spec: DecorSpec::default(),
            keyframes: Vec::new(),
            param_keyframes: Vec::new(),
        }
    }

    /// Evaluate the decor at `time_ms`.
    ///
    /// Returns `None` outside `[start_ms, end_ms]`. The state defaults to
    /// fully visible / identity when the state track is empty or does not
    /// cover `time_ms`.
    pub fn sample(&self, time_ms: i64) -> Option<AnimatedDecor> {
        if time_ms < self.start_ms || time_ms > self.end_ms {
            return None;
        }

        let state = if self.keyframes.is_empty() {
            AnimatedState::default()
        } else {
            sample_state(&self.keyframes, time_ms).unwrap_or_default()
        };

        let mut params = self.spec.params.clone();
        for track in &self.param_keyframes {
            if let Some(value) = track.sample(time_ms) {
                params.insert(track.key.clone(), value);
            }
        }

        Some(AnimatedDecor {
            decor_id: self.decor_id,
            target: self.target,
            state,
            params,
        })
    }

    /// Validate the structural invariants:
    /// - the active window is not inverted;
    /// - every parameter track's keyframes are strictly ascending.
    pub fn validate(&self) -> Result<(), CkError> {
        if self.start_ms > self.end_ms {
            return Err(CkError::new(
                codes::WINDOW_INVERTED,
                format!(
                    "decor {} window is inverted: {} > {}",
                    self.decor_id, self.start_ms, self.end_ms
                ),
            ));
        }
        for window in self.keyframes.windows(2) {
            if window[1].time_ms <= window[0].time_ms {
                return Err(CkError::new(
                    codes::PARAM_KEYFRAMES_UNSORTED,
                    format!("decor {} state keyframes are not ascending", self.decor_id),
                ));
            }
        }
        for track in &self.param_keyframes {
            for window in track.keyframes.windows(2) {
                if window[1].time_ms <= window[0].time_ms {
                    return Err(CkError::new(
                        codes::PARAM_KEYFRAMES_UNSORTED,
                        format!(
                            "decor {} param track '{}' keyframes are not ascending",
                            self.decor_id, track.key
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Evaluated decor snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedDecor {
    /// Stable decor id.
    pub decor_id: u64,
    /// Decor target.
    pub target: DecorTarget,
    /// Evaluated visible / opacity / transform.
    pub state: AnimatedState,
    /// Static params with any active parameter tracks applied.
    pub params: BTreeMap<String, DecorValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: f64) -> DecorValue {
        DecorValue::Number(value)
    }

    #[test]
    fn number_interpolates() {
        assert_eq!(
            DecorValue::lerp(&number(0.0), &number(10.0), 0.25),
            number(2.5)
        );
    }

    #[test]
    fn bool_and_text_step() {
        assert_eq!(
            DecorValue::lerp(&DecorValue::Bool(false), &DecorValue::Bool(true), 0.9),
            DecorValue::Bool(false)
        );
        assert_eq!(
            DecorValue::lerp(
                &DecorValue::Text("a".into()),
                &DecorValue::Text("b".into()),
                0.9
            ),
            DecorValue::Text("a".into())
        );
    }

    #[test]
    fn color_interpolates_components() {
        let a = DecorValue::Color(CkColor::new(0, 0, 0, 0));
        let b = DecorValue::Color(CkColor::new(255, 100, 50, 255));
        let mid = DecorValue::lerp(&a, &b, 0.5);
        assert_eq!(mid, DecorValue::Color(CkColor::new(128, 50, 25, 128)));
    }

    #[test]
    fn same_variant_anchor_interpolates() {
        let a = DecorValue::Anchor(DecorAnchor::InkBox { u: 0.0, v: 1.0 });
        let b = DecorValue::Anchor(DecorAnchor::InkBox { u: 1.0, v: 0.0 });
        assert_eq!(
            DecorValue::lerp(&a, &b, 0.5),
            DecorValue::Anchor(DecorAnchor::InkBox { u: 0.5, v: 0.5 })
        );
    }

    #[test]
    fn anchor_variant_switch_steps() {
        let a = DecorValue::Anchor(DecorAnchor::Box { u: 0.0, v: 0.0 });
        let b = DecorValue::Anchor(DecorAnchor::InkBox { u: 1.0, v: 1.0 });
        assert_eq!(DecorValue::lerp(&a, &b, 0.9), a);
    }

    #[test]
    fn number_list_length_change_steps() {
        let a = DecorValue::Numbers(vec![0.0, 1.0]);
        let b = DecorValue::Numbers(vec![1.0, 0.0, 0.5]);
        assert_eq!(DecorValue::lerp(&a, &b, 0.9), a);
    }

    #[test]
    fn param_track_samples_and_falls_back() {
        let track = DecorParamTrack::new(
            "rate",
            vec![
                DecorParamKeyframe {
                    time_ms: 0,
                    value: number(0.0),
                    easing: Easing::Linear,
                },
                DecorParamKeyframe {
                    time_ms: 100,
                    value: number(1.0),
                    easing: Easing::Linear,
                },
            ],
        );

        assert_eq!(track.sample(50), Some(number(0.5)));
        // Outside the track range the caller falls back to the static value.
        assert_eq!(track.sample(101), None);
        assert_eq!(DecorParamTrack::new("empty", vec![]).sample(0), None);
    }

    #[test]
    fn decor_sample_applies_param_tracks() {
        let mut item = DecorItem::new(1, DecorTarget::Line { line_index: 0 });
        item.start_ms = 0;
        item.end_ms = 200;
        item.spec.kind = "particle".to_string();
        item.spec.params.insert("rate".to_string(), number(10.0));
        item.param_keyframes.push(DecorParamTrack::new(
            "rate",
            vec![
                DecorParamKeyframe {
                    time_ms: 0,
                    value: number(0.0),
                    easing: Easing::Linear,
                },
                DecorParamKeyframe {
                    time_ms: 100,
                    value: number(100.0),
                    easing: Easing::Linear,
                },
            ],
        ));

        let sampled = item.sample(50).unwrap();
        assert_eq!(sampled.decor_id, 1);
        assert_eq!(sampled.params.get("rate"), Some(&number(50.0)));

        // Before/after the param track the static value wins.
        assert_eq!(item.sample(150).unwrap().params.get("rate"), Some(&number(10.0)));
        // Outside the decor window nothing is produced.
        assert!(item.sample(201).is_none());
    }

    #[test]
    fn decor_sample_defaults_state_when_no_track() {
        let mut item = DecorItem::new(2, DecorTarget::Main { line_index: 0, char_index: 3 });
        item.start_ms = 0;
        item.end_ms = 10;
        let sampled = item.sample(5).unwrap();
        assert!(sampled.state.visible);
        assert_eq!(sampled.state.opacity, 1.0);
        assert_eq!(
            sampled.target,
            DecorTarget::Main {
                line_index: 0,
                char_index: 3
            }
        );
    }

    #[test]
    fn validate_checks_windows_and_tracks() {
        let mut item = DecorItem::new(3, DecorTarget::Line { line_index: 0 });
        item.start_ms = 10;
        item.end_ms = 0;
        assert_eq!(item.validate().unwrap_err().code, codes::WINDOW_INVERTED);

        let mut ok = DecorItem::new(4, DecorTarget::Line { line_index: 0 });
        ok.start_ms = 0;
        ok.end_ms = 10;
        assert!(ok.validate().is_ok());
    }
}
