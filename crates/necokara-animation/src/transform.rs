//! 3D transform used by all animation targets.

/// A 3D transform. 2D is a subset: only x/y translation and z-axis rotation.
///
/// The transform is applied around a normalized origin inside the target's
/// bounding box:
///
/// ```text
/// M = T(origin) * T(translate) * R * Skew * S * T(-origin)
/// ```
///
/// Rotation angles are **not** normalized: scripts emit continuous angles
/// (e.g. `0 -> 720`) so linear interpolation yields a full spin instead of a
/// wrap-around.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3D {
    /// Translation in pixels.
    pub translate_x_px: f64,
    pub translate_y_px: f64,
    pub translate_z_px: f64,

    /// Scale; 1.0 = original size.
    pub scale_x: f64,
    pub scale_y: f64,
    pub scale_z: f64,

    /// Rotation around model axes, degrees.
    pub rotate_x_deg: f64,
    pub rotate_y_deg: f64,
    pub rotate_z_deg: f64,

    /// Skew / shear in degrees. Keeps parallel lines parallel; used for
    /// italic-like leaning. Not used by the first presets, kept for a complete
    /// transform API.
    pub skew_x_deg: f64,
    pub skew_y_deg: f64,

    /// Transform origin, normalized inside the target bounding box.
    /// `(0.5, 0.5, 0.5)` = center of the target.
    pub origin_x: f64,
    pub origin_y: f64,
    pub origin_z: f64,

    /// Perspective distance in pixels; `None` = orthographic projection.
    /// Needed for `rotate_x` / `rotate_y` to look like a real flip.
    pub perspective_px: Option<f64>,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            translate_x_px: 0.0,
            translate_y_px: 0.0,
            translate_z_px: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            scale_z: 1.0,
            rotate_x_deg: 0.0,
            rotate_y_deg: 0.0,
            rotate_z_deg: 0.0,
            skew_x_deg: 0.0,
            skew_y_deg: 0.0,
            origin_x: 0.5,
            origin_y: 0.5,
            origin_z: 0.5,
            perspective_px: None,
        }
    }
}

impl Transform3D {
    /// Whether this transform leaves the target untouched.
    pub fn is_identity(&self) -> bool {
        self.translate_x_px == 0.0
            && self.translate_y_px == 0.0
            && self.translate_z_px == 0.0
            && self.scale_x == 1.0
            && self.scale_y == 1.0
            && self.scale_z == 1.0
            && self.rotate_x_deg == 0.0
            && self.rotate_y_deg == 0.0
            && self.rotate_z_deg == 0.0
            && self.skew_x_deg == 0.0
            && self.skew_y_deg == 0.0
    }

    /// Component-wise linear interpolation from `a` to `b` at `t` in `0..=1`.
    ///
    /// `perspective_px` is interpolated when both sides define it; when only
    /// one side does, it steps (keeps `a` until the next keyframe).
    pub fn lerp(a: &Self, b: &Self, t: f64) -> Self {
        Self {
            translate_x_px: lerp(a.translate_x_px, b.translate_x_px, t),
            translate_y_px: lerp(a.translate_y_px, b.translate_y_px, t),
            translate_z_px: lerp(a.translate_z_px, b.translate_z_px, t),
            scale_x: lerp(a.scale_x, b.scale_x, t),
            scale_y: lerp(a.scale_y, b.scale_y, t),
            scale_z: lerp(a.scale_z, b.scale_z, t),
            rotate_x_deg: lerp(a.rotate_x_deg, b.rotate_x_deg, t),
            rotate_y_deg: lerp(a.rotate_y_deg, b.rotate_y_deg, t),
            rotate_z_deg: lerp(a.rotate_z_deg, b.rotate_z_deg, t),
            skew_x_deg: lerp(a.skew_x_deg, b.skew_x_deg, t),
            skew_y_deg: lerp(a.skew_y_deg, b.skew_y_deg, t),
            origin_x: lerp(a.origin_x, b.origin_x, t),
            origin_y: lerp(a.origin_y, b.origin_y, t),
            origin_z: lerp(a.origin_z, b.origin_z, t),
            perspective_px: match (a.perspective_px, b.perspective_px) {
                (Some(x), Some(y)) => Some(lerp(x, y, t)),
                (x, _) => x,
            },
        }
    }
}

/// Linear interpolation between two `f64` values.
pub(crate) fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_identity() {
        assert!(Transform3D::default().is_identity());
    }

    #[test]
    fn non_identity_is_detected() {
        let t = Transform3D {
            translate_x_px: 1.0,
            ..Transform3D::default()
        };
        assert!(!t.is_identity());
    }

    #[test]
    fn lerp_hits_both_ends() {
        let a = Transform3D::default();
        let b = Transform3D {
            translate_x_px: 100.0,
            rotate_z_deg: 720.0,
            scale_x: 2.0,
            ..Transform3D::default()
        };

        let start = Transform3D::lerp(&a, &b, 0.0);
        assert_eq!(start, a);

        let end = Transform3D::lerp(&a, &b, 1.0);
        assert_eq!(end, b);

        let mid = Transform3D::lerp(&a, &b, 0.5);
        assert!((mid.translate_x_px - 50.0).abs() < 1e-9);
        assert!((mid.rotate_z_deg - 360.0).abs() < 1e-9);
        assert!((mid.scale_x - 1.5).abs() < 1e-9);
    }

    #[test]
    fn perspective_interpolates_only_when_both_defined() {
        let mut a = Transform3D::default();
        let mut b = Transform3D::default();
        a.perspective_px = Some(1000.0);
        b.perspective_px = Some(2000.0);
        assert_eq!(
            Transform3D::lerp(&a, &b, 0.5).perspective_px,
            Some(1500.0)
        );

        // One side missing: step (keeps `a`).
        b.perspective_px = None;
        assert_eq!(Transform3D::lerp(&a, &b, 0.5).perspective_px, Some(1000.0));
    }
}
