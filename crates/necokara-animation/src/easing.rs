//! Easing curves applied between two keyframes.

/// Easing applied from one keyframe to the next.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Easing {
    /// Linear progress.
    #[default]
    Linear,
    /// Quadratic ease-in (slow start).
    QuadraticIn,
    /// Quadratic ease-out (slow end).
    QuadraticOut,
    /// Quadratic ease-in-out.
    QuadraticInOut,
    /// Explicit cubic bezier control points, CSS-timing-function style.
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
}

impl Easing {
    /// Map normalized time `t` (`0..=1`) to eased progress (`0..=1`).
    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::QuadraticIn => t * t,
            Easing::QuadraticOut => t * (2.0 - t),
            Easing::QuadraticInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    let u = -2.0 * t + 2.0;
                    1.0 - (u * u) / 2.0
                }
            }
            Easing::CubicBezier { x1, y1, x2, y2 } => {
                // Solve x(u) = t for u (Newton-Raphson with a bisection
                // fallback), then evaluate y(u).
                let x = solve_bezier(x1, x2, t);
                bezier(y1, y2, x)
            }
        }
    }
}

/// One cubic bezier component with fixed endpoints 0 and 1.
fn bezier(p1: f64, p2: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    3.0 * v * v * u * p1 + 3.0 * v * u * u * p2 + u * u * u
}

/// Derivative of the cubic bezier component.
fn bezier_derivative(p1: f64, p2: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    3.0 * v * v * p1 + 6.0 * v * u * (p2 - p1) + 3.0 * u * u * (1.0 - p2)
}

/// Find the bezier parameter `u` whose x component equals `x`.
fn solve_bezier(x1: f64, x2: f64, x: f64) -> f64 {
    // Newton-Raphson.
    let mut u = x;
    for _ in 0..8 {
        let error = bezier(x1, x2, u) - x;
        if error.abs() < 1e-7 {
            return u;
        }
        let d = bezier_derivative(x1, x2, u);
        if d.abs() < 1e-7 {
            break;
        }
        u -= error / d;
    }

    // Bisection fallback for flat / out-of-range control points.
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    let mut u = x.clamp(0.0, 1.0);
    for _ in 0..32 {
        let value = bezier(x1, x2, u);
        if (value - x).abs() < 1e-7 {
            break;
        }
        if value < x {
            lo = u;
        } else {
            hi = u;
        }
        u = (lo + hi) / 2.0;
    }
    u
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_easings_pin_the_endpoints() {
        let easings = [
            Easing::Linear,
            Easing::QuadraticIn,
            Easing::QuadraticOut,
            Easing::QuadraticInOut,
            Easing::CubicBezier {
                x1: 0.42,
                y1: 0.0,
                x2: 0.58,
                y2: 1.0,
            },
        ];
        for easing in easings {
            assert!((easing.apply(0.0) - 0.0).abs() < 1e-9, "{easing:?}");
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-9, "{easing:?}");
        }
    }

    #[test]
    fn linear_is_identity() {
        assert!((Easing::Linear.apply(0.37) - 0.37).abs() < 1e-9);
    }

    #[test]
    fn quadratic_shapes() {
        assert!((Easing::QuadraticIn.apply(0.5) - 0.25).abs() < 1e-9);
        assert!((Easing::QuadraticOut.apply(0.5) - 0.75).abs() < 1e-9);
        // In-out is symmetric around the midpoint.
        let a = Easing::QuadraticInOut.apply(0.25);
        let b = Easing::QuadraticInOut.apply(0.75);
        assert!((a + b - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cubic_bezier_is_monotonic_and_eased() {
        let ease = Easing::CubicBezier {
            x1: 0.42,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        };
        let mid = ease.apply(0.5);
        assert!((mid - 0.5).abs() < 1e-3);
        // Ease-in-out starts slower than linear.
        assert!(ease.apply(0.25) < 0.25);
    }

    #[test]
    fn out_of_range_input_is_clamped() {
        assert_eq!(Easing::Linear.apply(-1.0), 0.0);
        assert_eq!(Easing::Linear.apply(2.0), 1.0);
    }
}
