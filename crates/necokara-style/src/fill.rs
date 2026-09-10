//! Fill and dual-state fill types.
//!
//! A `Fill` describes how a glyph area is painted: solid color, a smooth
//! linear gradient, a linear segment fill (hard color boundaries), or an
//! image referenced by a `crl://` material-tree URL.
//!
//! A [`DualFill`] bundles the two states used by karaoke rendering: `normal`
//! is the unsung state and `active` is the sung/highlighted state.

use necokara_error::CkError;
use necokara_path::parse_crl;

use crate::color::CkColor;

/// One stop inside a linear fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillStop {
    /// Stop position in the `0.0..=1.0` range.
    pub position: f64,
    /// Stop color.
    pub color: CkColor,
}

/// How a glyph area is painted.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    /// A single opaque/semi-transparent color.
    Solid {
        /// Fill color.
        color: CkColor,
    },
    /// Smooth linear gradient between stops.
    LinearGradient {
        /// Direction in degrees; `0` = left to right, `90` = top to bottom.
        angle_deg: f64,
        /// Gradient stops.
        stops: Vec<FillStop>,
    },
    /// Linear segmented fill with hard boundaries (no interpolation).
    LinearSegments {
        /// Direction in degrees; `0` = left to right, `90` = top to bottom.
        angle_deg: f64,
        /// Hard-edged segment stops.
        stops: Vec<FillStop>,
    },
    /// Image fill referenced through the material tree.
    Image {
        /// A `crl://` tree path URL.
        crl: String,
    },
}

impl Fill {
    /// Build a solid fill.
    pub fn solid(color: CkColor) -> Self {
        Self::Solid { color }
    }

    /// Build a smooth linear gradient fill.
    pub fn linear_gradient(angle_deg: f64, stops: Vec<FillStop>) -> Self {
        Self::LinearGradient { angle_deg, stops }
    }

    /// Build a linear segmented fill with hard color boundaries.
    pub fn linear_segments(angle_deg: f64, stops: Vec<FillStop>) -> Self {
        Self::LinearSegments { angle_deg, stops }
    }

    /// Build an image fill from a `crl://` URL.
    ///
    /// The URL is validated with `necokara-path`'s parser.
    pub fn image(crl: impl Into<String>) -> Result<Self, CkError> {
        let crl = crl.into();
        parse_crl(&crl)?;
        Ok(Self::Image { crl })
    }
}

/// Karaoke dual-state fill: unsung (`normal`) vs sung (`active`).
///
/// `active` is optional: `None` means the active state uses the same fill as
/// `normal`, i.e. there is no visual state change. The frontend/renderer
/// should treat this as `wipe = none`.
#[derive(Debug, Clone, PartialEq)]
pub struct DualFill {
    /// Unsung state.
    pub normal: Fill,
    /// Sung/highlighted state; `None` = same as `normal` (no wipe change).
    pub active: Option<Fill>,
}

impl DualFill {
    /// Build a dual-state fill.
    pub fn new(normal: Fill, active: Option<Fill>) -> Self {
        Self { normal, active }
    }

    /// Build a single-state fill where the active state equals `normal`.
    pub fn mono(normal: Fill) -> Self {
        Self {
            normal,
            active: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_fill_accepts_crl() {
        let fill = Fill::image("crl://images/brush_01").expect("valid crl");
        assert_eq!(
            fill,
            Fill::Image {
                crl: "crl://images/brush_01".to_string()
            }
        );
    }

    #[test]
    fn image_fill_rejects_non_crl() {
        assert!(Fill::image("C:/images/brush.png").is_err());
    }

    #[test]
    fn solid_and_dual_fill_constructors() {
        let normal = Fill::solid(CkColor::rgb(255, 255, 255));
        let active = Fill::solid(CkColor::rgb(255, 230, 0));
        let dual = DualFill::new(normal.clone(), Some(active.clone()));
        assert_eq!(dual.normal, normal);
        assert_eq!(dual.active, Some(active));
        assert_eq!(DualFill::mono(normal.clone()).active, None);
    }
}
