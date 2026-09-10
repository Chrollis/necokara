//! Ruby style: layout of the ruby (furigana) block relative to its base text.
//!
//! Ruby text appearance is intentionally *not* part of this style kind: ruby
//! characters carry their own [`CharacterStyle`](crate::CharacterStyle), as
//! decided in the style-layer design. `RubyStyle` only controls placement,
//! horizontal alignment, and the distance between ruby and base text.

use necokara_error::CkError;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// The built-in default ruby style has missing required fields.
    pub const DEFAULT_INCOMPLETE: &str = ck_code!("necokara-style", ruby, default_incomplete);
    /// The `based_on` chain contains a cycle.
    pub const BASED_ON_CYCLE: &str = ck_code!("necokara-style", ruby, based_on_cycle);
}

/// Which side of the base text the ruby block is placed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RubyPosition {
    /// Follow the default determined by text direction / layout.
    Auto,
    /// Place ruby above the base text.
    Above,
    /// Place ruby below the base text.
    Below,
}

/// Horizontal alignment of ruby characters against their base characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RubyAlign {
    /// Automatic alignment (usually per-character centering).
    Auto,
    /// Align ruby to the left of the base span.
    Left,
    /// Center ruby over the base span.
    Center,
    /// Align ruby to the right of the base span.
    Right,
    /// Spread ruby evenly across the base span.
    SpaceBetween,
    /// Spread ruby evenly, including space at both ends.
    SpaceAround,
}

/// A named ruby layout style entry.
///
/// Optional fields follow the same inheritance convention as character
/// styles: `None` means "inherit from `based_on` or from the built-in default".
#[derive(Debug, Clone, PartialEq)]
pub struct RubyStyle {
    /// Unique style id within the ruby style table.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Parent style id; `None` means the built-in default.
    pub based_on: Option<String>,
    /// Hidden styles are anonymous/direct-format entries, not shown in the UI.
    pub hidden: bool,

    /// Placement side relative to base text.
    pub position: Option<RubyPosition>,
    /// Horizontal alignment of the ruby block.
    pub align: Option<RubyAlign>,
    /// Gap between ruby and base text in pixels.
    pub distance_px: Option<f64>,
}

impl RubyStyle {
    /// Build an empty ruby style with the given id and no explicit fields.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            based_on: None,
            hidden: false,
            position: None,
            align: None,
            distance_px: None,
        }
    }
}

/// Resolve a ruby style against the ruby style table and the built-in default.
///
/// Returns an error when the default is missing a required field or when the
/// `based_on` chain contains a cycle.
pub fn resolve(
    input: &RubyStyle,
    list: &[RubyStyle],
    default: &RubyStyle,
) -> Result<RubyStyle, CkError> {
    validate_default(default)?;

    let mut chain: Vec<&RubyStyle> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut current = input;

    loop {
        if seen.contains(&current.id.as_str()) {
            return Err(CkError::new(
                codes::BASED_ON_CYCLE,
                format!("based_on cycle detected at ruby style '{}'", current.id),
            ));
        }
        seen.push(current.id.as_str());
        chain.push(current);

        match current.based_on.as_deref() {
            None | Some("") => break,
            Some(base_id) => match list.iter().find(|style| style.id == base_id) {
                Some(base) => current = base,
                None => break,
            },
        }
    }

    let mut result = default.clone();
    for style in chain.iter().rev() {
        if let Some(position) = style.position {
            result.position = Some(position);
        }
        if let Some(align) = style.align {
            result.align = Some(align);
        }
        if let Some(distance_px) = style.distance_px {
            result.distance_px = Some(distance_px);
        }
    }

    result.id = input.id.clone();
    result.name = input.name.clone();
    result.based_on = input.based_on.clone();
    result.hidden = input.hidden;
    Ok(result)
}

/// Ensure the built-in default contains all required ruby layout values.
fn validate_default(default: &RubyStyle) -> Result<(), CkError> {
    let mut missing: Vec<&'static str> = Vec::new();

    if default.position.is_none() {
        missing.push("position");
    }
    if default.align.is_none() {
        missing.push("align");
    }
    if default.distance_px.is_none() {
        missing.push("distance_px");
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(CkError::new(
            codes::DEFAULT_INCOMPLETE,
            format!("default ruby style is missing required fields: {}", missing.join(", ")),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_style() -> RubyStyle {
        RubyStyle {
            id: "style_ruby_default".to_string(),
            name: "Default".to_string(),
            based_on: None,
            hidden: false,
            position: Some(RubyPosition::Above),
            align: Some(RubyAlign::Center),
            distance_px: Some(6.0),
        }
    }

    #[test]
    fn resolve_fills_missing_fields_from_default() {
        let input = RubyStyle::new("style_ruby_input");
        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_ruby_input");
        assert_eq!(result.position, Some(RubyPosition::Above));
        assert_eq!(result.align, Some(RubyAlign::Center));
        assert_eq!(result.distance_px, Some(6.0));
    }

    #[test]
    fn resolve_input_overrides_default() {
        let mut input = RubyStyle::new("style_ruby_left");
        input.position = Some(RubyPosition::Below);
        input.align = Some(RubyAlign::Left);

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.position, Some(RubyPosition::Below));
        assert_eq!(result.align, Some(RubyAlign::Left));
        assert_eq!(result.distance_px, Some(6.0));
    }

    #[test]
    fn resolve_follows_based_on_chain() {
        let default = default_style();

        let mut base = RubyStyle::new("style_ruby_base");
        base.position = Some(RubyPosition::Above);
        base.distance_px = Some(10.0);

        let mut derived = RubyStyle::new("style_ruby_derived");
        derived.based_on = Some("style_ruby_base".to_string());
        derived.align = Some(RubyAlign::Right);

        let list = vec![base, derived];
        let result = resolve(&list[1], &list, &default).expect("valid table");

        assert_eq!(result.id, "style_ruby_derived");
        assert_eq!(result.position, Some(RubyPosition::Above));
        assert_eq!(result.align, Some(RubyAlign::Right));
        assert_eq!(result.distance_px, Some(10.0));
    }

    #[test]
    fn resolve_missing_based_on_uses_default() {
        let mut input = RubyStyle::new("style_ruby_input");
        input.based_on = Some("style_ruby_missing".to_string());

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_ruby_input");
        assert_eq!(result.position, Some(RubyPosition::Above));
    }

    #[test]
    fn resolve_default_incomplete_errors() {
        let mut incomplete = default_style();
        incomplete.align = None;

        let err = resolve(&RubyStyle::new("style_ruby_input"), &[], &incomplete)
            .expect_err("incomplete default must fail");
        assert_eq!(err.code, codes::DEFAULT_INCOMPLETE);
    }

    #[test]
    fn resolve_cycle_errors() {
        let mut a = RubyStyle::new("style_ruby_a");
        a.based_on = Some("style_ruby_b".to_string());

        let mut b = RubyStyle::new("style_ruby_b");
        b.based_on = Some("style_ruby_a".to_string());

        let list = vec![a, b];
        let err = resolve(&list[0], &list, &default_style()).expect_err("cycle must fail");
        assert_eq!(err.code, codes::BASED_ON_CYCLE);
    }

    #[test]
    fn align_variants_include_left_right() {
        assert_eq!(RubyAlign::Left as u8, 1);
        assert_eq!(RubyAlign::Right as u8, 3);
    }
}
