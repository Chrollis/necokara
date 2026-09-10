//! Wire format (JSON) for the animation script calls.
//!
//! Core types stay serde-free: this module owns the mirror DTOs and the
//! explicit conversions between them and the core types. See
//! `develop/DESIGN_v0.4_animation.md` §4 for the protocol.
//!
//! - Request: [`request_value`] builds the `{"fps", "lines": [...]}` document
//!   written to the script's stdin.
//! - Response: [`line_animations_from_value`] converts the unwrapped payload
//!   (`{"lines": [...]}`) back into [`LineAnimation`]s, validating each one
//!   through the core invariant checks.
//!
//! `decors` is parsed but rejected while non-empty: the decor wire format is
//! not settled yet, so scripts must emit an empty array for now.

use necokara_error::CkError;
use serde::{Deserialize, Serialize};

use crate::easing::Easing;
use crate::keyframe::{LineAnimation, MainKeyframes, RubyKeyframes, StateKeyframe};
use crate::script::ScriptLineInput;
use crate::state::AnimatedState;
use crate::transform::Transform3D;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// The script's response did not match the wire format.
    pub const INVALID_PAYLOAD: &str = ck_code!("necokara-animation", json, invalid_payload);
    /// The request could not be serialized (should not happen for these types).
    pub const REQUEST_FAILED: &str = ck_code!("necokara-animation", json, request_failed);
    /// The response carried decor instructions, which are not supported yet.
    pub const UNSUPPORTED_DECORS: &str = ck_code!("necokara-animation", json, unsupported_decors);
}

// ---------------------------------------------------------------------------
// Request DTOs (Rust -> script)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AnimationRequest {
    fps: f64,
    lines: Vec<LineRequest>,
}

#[derive(Debug, Serialize)]
struct LineRequest {
    line_index: usize,
    script: String,
    line_start_ms: i64,
    line_end_ms: i64,
    lead_in_ms: i64,
    lead_out_ms: i64,
    main: Vec<MainRequest>,
    ruby: Vec<RubyRequest>,
}

#[derive(Debug, Serialize)]
struct MainRequest {
    char_index: usize,
    start_ms: i64,
    duration_ms: u32,
}

#[derive(Debug, Serialize)]
struct RubyRequest {
    ruby_index: usize,
    start_ms: i64,
    duration_ms: u32,
}

/// Build the request document written to a script's stdin.
pub fn request_value(
    fps: f64,
    lines: &[ScriptLineInput],
) -> Result<serde_json::Value, CkError> {
    let request = AnimationRequest {
        fps,
        lines: lines
            .iter()
            .map(|line| LineRequest {
                line_index: line.line_index,
                script: line.script.clone(),
                line_start_ms: line.line_start_ms,
                line_end_ms: line.line_end_ms,
                lead_in_ms: line.lead_in_ms,
                lead_out_ms: line.lead_out_ms,
                main: line
                    .main
                    .iter()
                    .map(|cell| MainRequest {
                        char_index: cell.char_index,
                        start_ms: cell.start_ms,
                        duration_ms: cell.duration_ms,
                    })
                    .collect(),
                ruby: line
                    .ruby
                    .iter()
                    .map(|cell| RubyRequest {
                        ruby_index: cell.ruby_index,
                        start_ms: cell.start_ms,
                        duration_ms: cell.duration_ms,
                    })
                    .collect(),
            })
            .collect(),
    };

    serde_json::to_value(&request).map_err(|e| {
        CkError::new(codes::REQUEST_FAILED, "could not serialize the animation request")
            .with_source(e.to_string())
    })
}

// ---------------------------------------------------------------------------
// Response DTOs (script -> Rust)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct LineAnimationsPayload {
    lines: Vec<LineAnimationDto>,
}

#[derive(Debug, Deserialize)]
struct LineAnimationDto {
    line_index: usize,
    script: String,
    #[serde(default)]
    animation_in_ms: i64,
    #[serde(default)]
    animation_out_ms: i64,
    line: Vec<StateKeyframeDto>,
    #[serde(default)]
    main: Vec<MainKeyframesDto>,
    #[serde(default)]
    ruby: Vec<RubyKeyframesDto>,
    #[serde(default)]
    decors: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct MainKeyframesDto {
    char_index: usize,
    keyframes: Vec<StateKeyframeDto>,
}

#[derive(Debug, Deserialize)]
struct RubyKeyframesDto {
    ruby_index: usize,
    keyframes: Vec<StateKeyframeDto>,
}

#[derive(Debug, Deserialize)]
struct StateKeyframeDto {
    time_ms: i64,
    state: StateDto,
    #[serde(default)]
    easing: EasingDto,
}

#[derive(Debug, Deserialize)]
struct StateDto {
    visible: bool,
    opacity: f64,
    transform: TransformDto,
}

#[derive(Debug, Deserialize)]
struct TransformDto {
    translate_x_px: f64,
    translate_y_px: f64,
    translate_z_px: f64,
    scale_x: f64,
    scale_y: f64,
    scale_z: f64,
    rotate_x_deg: f64,
    rotate_y_deg: f64,
    rotate_z_deg: f64,
    skew_x_deg: f64,
    skew_y_deg: f64,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
    perspective_px: Option<f64>,
}

/// `"linear" | "quadratic_in" | ... | { "cubic_bezier": { ... } }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EasingDto {
    Named(String),
    Cubic(BezierWrapper),
}

impl Default for EasingDto {
    fn default() -> Self {
        Self::Named("linear".to_string())
    }
}

#[derive(Debug, Deserialize)]
struct BezierWrapper {
    cubic_bezier: BezierDto,
}

#[derive(Debug, Deserialize)]
struct BezierDto {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl EasingDto {
    /// Convert to the core easing, rejecting unknown names.
    fn into_easing(self) -> Result<Easing, CkError> {
        match self {
            EasingDto::Named(name) => match name.as_str() {
                "linear" => Ok(Easing::Linear),
                "quadratic_in" => Ok(Easing::QuadraticIn),
                "quadratic_out" => Ok(Easing::QuadraticOut),
                "quadratic_in_out" => Ok(Easing::QuadraticInOut),
                other => Err(invalid_payload(format!("unknown easing '{other}'"))),
            },
            EasingDto::Cubic(wrapper) => Ok(Easing::CubicBezier {
                x1: wrapper.cubic_bezier.x1,
                y1: wrapper.cubic_bezier.y1,
                x2: wrapper.cubic_bezier.x2,
                y2: wrapper.cubic_bezier.y2,
            }),
        }
    }
}

impl StateDto {
    fn into_state(self) -> AnimatedState {
        AnimatedState {
            visible: self.visible,
            opacity: self.opacity,
            transform: Transform3D {
                translate_x_px: self.transform.translate_x_px,
                translate_y_px: self.transform.translate_y_px,
                translate_z_px: self.transform.translate_z_px,
                scale_x: self.transform.scale_x,
                scale_y: self.transform.scale_y,
                scale_z: self.transform.scale_z,
                rotate_x_deg: self.transform.rotate_x_deg,
                rotate_y_deg: self.transform.rotate_y_deg,
                rotate_z_deg: self.transform.rotate_z_deg,
                skew_x_deg: self.transform.skew_x_deg,
                skew_y_deg: self.transform.skew_y_deg,
                origin_x: self.transform.origin_x,
                origin_y: self.transform.origin_y,
                origin_z: self.transform.origin_z,
                perspective_px: self.transform.perspective_px,
            },
        }
    }
}

impl StateKeyframeDto {
    fn into_keyframe(self) -> Result<StateKeyframe, CkError> {
        Ok(StateKeyframe {
            time_ms: self.time_ms,
            state: self.state.into_state(),
            easing: self.easing.into_easing()?,
        })
    }
}

impl LineAnimationDto {
    fn into_line_animation(self) -> Result<LineAnimation, CkError> {
        if !self.decors.is_empty() {
            return Err(CkError::new(
                codes::UNSUPPORTED_DECORS,
                format!(
                    "line {} returned {} decor(s); the decor wire format is not supported yet",
                    self.line_index,
                    self.decors.len()
                ),
            ));
        }

        let line = self
            .line
            .into_iter()
            .map(StateKeyframeDto::into_keyframe)
            .collect::<Result<Vec<_>, _>>()?;
        let main = self
            .main
            .into_iter()
            .map(|track| {
                Ok(MainKeyframes {
                    char_index: track.char_index,
                    keyframes: track
                        .keyframes
                        .into_iter()
                        .map(StateKeyframeDto::into_keyframe)
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, CkError>>()?;
        let ruby = self
            .ruby
            .into_iter()
            .map(|track| {
                Ok(RubyKeyframes {
                    ruby_index: track.ruby_index,
                    keyframes: track
                        .keyframes
                        .into_iter()
                        .map(StateKeyframeDto::into_keyframe)
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, CkError>>()?;

        let animation = LineAnimation::new(
            self.line_index,
            self.script,
            self.animation_in_ms,
            self.animation_out_ms,
            line,
        )
        .with_main(main)
        .with_ruby(ruby);

        // Reuse the core invariants (script non-empty, line track non-empty,
        // times ascending, indices unique).
        animation.validate()?;
        Ok(animation)
    }
}

/// Convert an unwrapped script payload into line animations.
///
/// The payload is the value returned by `necokara_spawn`'s envelope, i.e.
/// `{"lines": [...]}`.
pub fn line_animations_from_value(
    value: serde_json::Value,
) -> Result<Vec<LineAnimation>, CkError> {
    let payload: LineAnimationsPayload = serde_json::from_value(value).map_err(|e| {
        CkError::new(
            codes::INVALID_PAYLOAD,
            "animation response did not match the wire format",
        )
        .with_source(e.to_string())
    })?;

    payload
        .lines
        .into_iter()
        .map(LineAnimationDto::into_line_animation)
        .collect()
}

/// Build the error for a malformed payload.
fn invalid_payload(message: impl Into<String>) -> CkError {
    CkError::new(codes::INVALID_PAYLOAD, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{MainCharInput, RubyCharInput};

    fn line_input() -> ScriptLineInput {
        ScriptLineInput {
            line_index: 3,
            script: "FadeIn".to_string(),
            line_start_ms: 12000,
            line_end_ms: 15600,
            lead_in_ms: 200,
            lead_out_ms: 300,
            main: vec![MainCharInput {
                char_index: 8,
                start_ms: 12100,
                duration_ms: 300,
            }],
            ruby: vec![RubyCharInput {
                ruby_index: 0,
                start_ms: 12100,
                duration_ms: 150,
            }],
        }
    }

    #[test]
    fn request_value_has_the_documented_shape() {
        let value = request_value(30.0, &[line_input()]).expect("serialize");

        assert_eq!(value["fps"], 30.0);
        let line = &value["lines"][0];
        assert_eq!(line["line_index"], 3);
        assert_eq!(line["script"], "FadeIn");
        assert_eq!(line["line_start_ms"], 12000);
        assert_eq!(line["line_end_ms"], 15600);
        assert_eq!(line["lead_in_ms"], 200);
        assert_eq!(line["lead_out_ms"], 300);
        assert_eq!(line["main"][0]["char_index"], 8);
        assert_eq!(line["main"][0]["start_ms"], 12100);
        assert_eq!(line["main"][0]["duration_ms"], 300);
        assert_eq!(line["ruby"][0]["ruby_index"], 0);
        assert_eq!(line["ruby"][0]["duration_ms"], 150);
    }

    /// Minimal full-state document, matching what the Python helper emits.
    fn state_json(opacity: f64) -> serde_json::Value {
        serde_json::json!({
            "visible": true,
            "opacity": opacity,
            "transform": {
                "translate_x_px": 0.0, "translate_y_px": 0.0, "translate_z_px": 0.0,
                "scale_x": 1.0, "scale_y": 1.0, "scale_z": 1.0,
                "rotate_x_deg": 0.0, "rotate_y_deg": 0.0, "rotate_z_deg": 0.0,
                "skew_x_deg": 0.0, "skew_y_deg": 0.0,
                "origin_x": 0.5, "origin_y": 0.5, "origin_z": 0.5,
                "perspective_px": null
            }
        })
    }

    fn payload_json() -> serde_json::Value {
        serde_json::json!({
            "lines": [{
                "line_index": 3,
                "script": "FadeIn",
                "animation_in_ms": 200,
                "animation_out_ms": 0,
                "line": [
                    { "time_ms": 11800, "state": state_json(0.0), "easing": "linear" },
                    { "time_ms": 12000, "state": state_json(1.0), "easing": "linear" }
                ],
                "main": [],
                "ruby": [],
                "decors": []
            }]
        })
    }

    #[test]
    fn parses_a_valid_payload() {
        let animations = line_animations_from_value(payload_json()).expect("valid payload");
        assert_eq!(animations.len(), 1);
        let anim = &animations[0];
        assert_eq!(anim.line_index, 3);
        assert_eq!(anim.script, "FadeIn");
        assert_eq!(anim.animation_in_ms, 200);
        assert_eq!(anim.line_window(), Some((11800, 12000)));
        assert_eq!(anim.sample(11900).unwrap().state.opacity, 0.5);
    }

    #[test]
    fn parses_named_and_bezier_easings() {
        let mut payload = payload_json();
        payload["lines"][0]["line"][0]["easing"] = serde_json::json!("quadratic_in");
        payload["lines"][0]["line"][1]["easing"] = serde_json::json!({
            "cubic_bezier": { "x1": 0.42, "y1": 0.0, "x2": 0.58, "y2": 1.0 }
        });
        let animations = line_animations_from_value(payload).expect("valid payload");
        assert_eq!(animations[0].line[0].easing, Easing::QuadraticIn);
        assert_eq!(
            animations[0].line[1].easing,
            Easing::CubicBezier {
                x1: 0.42,
                y1: 0.0,
                x2: 0.58,
                y2: 1.0
            }
        );
    }

    #[test]
    fn missing_easing_defaults_to_linear() {
        let mut payload = payload_json();
        payload["lines"][0]["line"][0]
            .as_object_mut()
            .unwrap()
            .remove("easing");
        let animations = line_animations_from_value(payload).expect("valid payload");
        assert_eq!(animations[0].line[0].easing, Easing::Linear);
    }

    #[test]
    fn unknown_easing_is_rejected() {
        let mut payload = payload_json();
        payload["lines"][0]["line"][0]["easing"] = serde_json::json!("bouncy");
        let err = line_animations_from_value(payload).unwrap_err();
        assert_eq!(err.code, codes::INVALID_PAYLOAD);
    }

    #[test]
    fn unsorted_keyframes_are_rejected() {
        let mut payload = payload_json();
        payload["lines"][0]["line"][0]["time_ms"] = serde_json::json!(12000);
        payload["lines"][0]["line"][1]["time_ms"] = serde_json::json!(11800);
        let err = line_animations_from_value(payload).unwrap_err();
        assert_eq!(err.code, crate::keyframe::codes::KEYFRAMES_UNSORTED);
    }

    #[test]
    fn empty_line_track_is_rejected() {
        let mut payload = payload_json();
        payload["lines"][0]["line"] = serde_json::json!([]);
        let err = line_animations_from_value(payload).unwrap_err();
        assert_eq!(err.code, crate::keyframe::codes::LINE_KEYFRAMES_EMPTY);
    }

    #[test]
    fn empty_script_is_rejected() {
        let mut payload = payload_json();
        payload["lines"][0]["script"] = serde_json::json!("");
        let err = line_animations_from_value(payload).unwrap_err();
        assert_eq!(err.code, crate::keyframe::codes::SCRIPT_EMPTY);
    }

    #[test]
    fn non_empty_decors_are_rejected_for_now() {
        let mut payload = payload_json();
        payload["lines"][0]["decors"] = serde_json::json!([{ "decor_id": 1 }]);
        let err = line_animations_from_value(payload).unwrap_err();
        assert_eq!(err.code, codes::UNSUPPORTED_DECORS);
    }

    #[test]
    fn payload_without_lines_is_invalid() {
        let err = line_animations_from_value(serde_json::json!({ "nope": true })).unwrap_err();
        assert_eq!(err.code, codes::INVALID_PAYLOAD);
    }
}
