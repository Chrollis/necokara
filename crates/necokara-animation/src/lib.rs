//! Necokara line-level preset animation.
//!
//! This crate is the animation layer of the core data model. It defines:
//!
//! - [`script_manifest`] / [`LineAnimAlloc`]: the dynamic set of animation
//!   scripts and which line runs which script, plus that line's `lead_in` /
//!   `lead_out` window extension;
//! - [`Transform3D`] / [`AnimatedState`]: the visual parameters a renderer
//!   consumes;
//! - [`LineAnimation`] with sparse [`StateKeyframe`] tracks at three parallel
//!   levels: line / main character / ruby character;
//! - [`DecorItem`] / [`DecorSpec`] / [`DecorAnchor`]: renderer-interpreted
//!   decoration instructions, including glyph-ink-relative placement;
//! - [`script`]: grouping lines by script and invoking the Python preset
//!   through `necokara-spawn` (stdin JSON / stdout envelope), with the wire
//!   format in [`json`].
//!
//! The script set is data-driven: `python/scripts/presets/manifest.json` lists
//! the available scripts and an allocation stores a script name (its file name
//! without `.py`). A Python script computes the keyframes per line from
//! `lyrics + line_index + script`; the WebGL renderer evaluates the sparse
//! tracks at playback time and draws. This crate never rasterizes anything and
//! never carries lyric text (indices and states only).

pub mod alloc;
pub mod decor;
pub mod easing;
pub mod json;
pub mod keyframe;
pub mod script;
pub mod script_manifest;
pub mod state;
pub mod transform;

pub use alloc::{LineAnimAlloc, LineAnimRun};
pub use decor::{
    AnimatedDecor, DecorAnchor, DecorItem, DecorParamKeyframe, DecorParamTrack, DecorSpec,
    DecorTarget, DecorValue,
};
pub use easing::Easing;
pub use json::{line_animations_from_value, request_value};
pub use keyframe::{sample_state, LineAnimation, MainKeyframes, RubyKeyframes, StateKeyframe};
pub use script::{
    group_by_script, run_groups, run_script, LineInputs, MainCharInput, RubyCharInput,
    ScriptLineInput,
};
pub use script_manifest::{
    load_script_manifest, parse_script_manifest, script_name_of, AnimationScript, LocalizedNames,
};
pub use state::{AnimatedLine, AnimatedMain, AnimatedRuby, AnimatedState};
pub use transform::Transform3D;
