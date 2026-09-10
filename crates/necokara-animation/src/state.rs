//! Animation state and the line / main / ruby target hierarchy.
//!
//! These are *evaluated* runtime snapshots (the state of a target at one point
//! in time). Storage and cross-IPC transport use the sparse keyframe tracks in
//! [`crate::keyframe`]; a renderer evaluates those into these snapshots.

use crate::transform::Transform3D;

/// Concrete animation state of one target at one point in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimatedState {
    /// When false the target is not drawn at all.
    pub visible: bool,
    /// Opacity; 0.0 = transparent, 1.0 = opaque.
    pub opacity: f64,
    /// 3D transform.
    pub transform: Transform3D,
}

impl Default for AnimatedState {
    fn default() -> Self {
        Self {
            visible: true,
            opacity: 1.0,
            transform: Transform3D::default(),
        }
    }
}

impl AnimatedState {
    /// Linear interpolation from `a` to `b` at `t` in `0..=1`.
    ///
    /// `visible` is a boolean and therefore steps: it keeps `a`'s value until
    /// the next keyframe.
    pub fn lerp(a: &Self, b: &Self, t: f64) -> Self {
        Self {
            visible: if t < 1.0 { a.visible } else { b.visible },
            opacity: crate::transform::lerp(a.opacity, b.opacity, t),
            transform: Transform3D::lerp(&a.transform, &b.transform, t),
        }
    }
}

/// Evaluated state of one main character.
///
/// `char_index` is a **full main character-stream index**; whitespace and `\n`
/// are included naturally. The animation layer never carries the character
/// itself, only the index; the renderer resolves indices to glyphs through the
/// lyrics model and does not draw whitespace / `\n` cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimatedMain {
    /// Full main character-stream index.
    pub char_index: usize,
    /// State of this character.
    pub state: AnimatedState,
}

/// Evaluated state of one ruby character.
///
/// It follows its owning main character's timing but is an independent state
/// (not a copy). The renderer maps `ruby_index` to its owner through
/// `WordAlloc::ruby_seg_lens`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimatedRuby {
    /// Full ruby character-stream index.
    pub ruby_index: usize,
    /// State of this ruby character.
    pub state: AnimatedState,
}

/// Evaluated state of one lyric line.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedLine {
    /// Line index (see `necokara_lyrics::Lyrics::line_ranges`).
    pub line_index: usize,
    /// Line-level state.
    pub state: AnimatedState,
    /// Sparse per-main-char states; missing indices inherit the line state.
    pub main: Vec<AnimatedMain>,
    /// Sparse per-ruby-char states; missing indices inherit the line state.
    pub ruby: Vec<AnimatedRuby>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_fully_visible_identity() {
        let state = AnimatedState::default();
        assert!(state.visible);
        assert_eq!(state.opacity, 1.0);
        assert!(state.transform.is_identity());
    }

    #[test]
    fn lerp_interpolates_opacity() {
        let a = AnimatedState::default();
        let b = AnimatedState {
            visible: true,
            opacity: 0.0,
            ..AnimatedState::default()
        };
        assert_eq!(AnimatedState::lerp(&a, &b, 0.25).opacity, 0.75);
        assert_eq!(AnimatedState::lerp(&a, &b, 1.0).opacity, 0.0);
    }

    #[test]
    fn lerp_steps_visibility() {
        let a = AnimatedState::default();
        let b = AnimatedState {
            visible: false,
            ..AnimatedState::default()
        };
        // Visibility keeps the previous value until the next keyframe.
        assert!(AnimatedState::lerp(&a, &b, 0.5).visible);
        assert!(!AnimatedState::lerp(&a, &b, 1.0).visible);
    }
}
