//! RGBA color type used by Necokara styles.

/// An RGBA color. Alpha `255` is fully opaque, matching CSS/PNG conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CkColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel (`255` = opaque).
    pub a: u8,
}

impl CkColor {
    /// Build an RGBA color.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Build an opaque RGB color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Replace the alpha channel.
    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_is_opaque() {
        let c = CkColor::rgb(1, 2, 3);
        assert_eq!(c.a, 255);
        assert_eq!(c.r, 1);
        assert_eq!(c.g, 2);
        assert_eq!(c.b, 3);
    }

    #[test]
    fn with_alpha_replaces_alpha_only() {
        let c = CkColor::rgb(10, 20, 30).with_alpha(128);
        assert_eq!(c, CkColor::new(10, 20, 30, 128));
    }
}
