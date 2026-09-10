//! Paragraph (line) style: horizontal text layout for one lyric line.
//!
//! In Necokara a lyric line is treated as one paragraph. Paragraph style
//! therefore only stores the line's horizontal alignment and indentation.
//! Line spacing is intentionally not part of this style kind; it belongs to
//! `PageStyle`.

use necokara_error::CkError;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// The built-in default paragraph style has missing required fields.
    pub const DEFAULT_INCOMPLETE: &str =
        ck_code!("necokara-style", paragraph, default_incomplete);
    /// The `based_on` chain contains a cycle.
    pub const BASED_ON_CYCLE: &str = ck_code!("necokara-style", paragraph, based_on_cycle);
}

/// Horizontal alignment of a lyric line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParagraphAlign {
    /// Align toward the inline-start edge (logical start).
    Start,
    /// Center the line.
    Center,
    /// Align toward the inline-end edge (logical end).
    End,
}

/// A named paragraph / line style entry.
///
/// Optional fields follow the same inheritance convention as the other style
/// kinds: `None` means "inherit from `based_on` or from the built-in default".
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphStyle {
    /// Unique style id within the paragraph style table.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Parent style id; `None` means the built-in default.
    pub based_on: Option<String>,
    /// Hidden styles are anonymous/direct-format entries, not shown in the UI.
    pub hidden: bool,

    /// Horizontal alignment of the line.
    pub align: Option<ParagraphAlign>,
    /// Left (inline-start) indent in pixels.
    pub left_indent_px: Option<f64>,
    /// Right (inline-end) indent in pixels.
    pub right_indent_px: Option<f64>,
}

impl ParagraphStyle {
    /// Build an empty paragraph style with the given id and no explicit fields.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            based_on: None,
            hidden: false,
            align: None,
            left_indent_px: None,
            right_indent_px: None,
        }
    }
}

/// Resolve a paragraph style against the paragraph style table and the
/// built-in default.
///
/// Returns an error when the default is missing a required field or when the
/// `based_on` chain contains a cycle.
pub fn resolve(
    input: &ParagraphStyle,
    list: &[ParagraphStyle],
    default: &ParagraphStyle,
) -> Result<ParagraphStyle, CkError> {
    validate_default(default)?;

    let mut chain: Vec<&ParagraphStyle> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut current = input;

    loop {
        if seen.contains(&current.id.as_str()) {
            return Err(CkError::new(
                codes::BASED_ON_CYCLE,
                format!(
                    "based_on cycle detected at paragraph style '{}'",
                    current.id
                ),
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
        if let Some(align) = style.align {
            result.align = Some(align);
        }
        if let Some(left_indent_px) = style.left_indent_px {
            result.left_indent_px = Some(left_indent_px);
        }
        if let Some(right_indent_px) = style.right_indent_px {
            result.right_indent_px = Some(right_indent_px);
        }
    }

    result.id = input.id.clone();
    result.name = input.name.clone();
    result.based_on = input.based_on.clone();
    result.hidden = input.hidden;
    Ok(result)
}

/// Ensure the built-in default contains all required paragraph layout values.
fn validate_default(default: &ParagraphStyle) -> Result<(), CkError> {
    let mut missing: Vec<&'static str> = Vec::new();

    if default.align.is_none() {
        missing.push("align");
    }
    if default.left_indent_px.is_none() {
        missing.push("left_indent_px");
    }
    if default.right_indent_px.is_none() {
        missing.push("right_indent_px");
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(CkError::new(
            codes::DEFAULT_INCOMPLETE,
            format!(
                "default paragraph style is missing required fields: {}",
                missing.join(", ")
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_style() -> ParagraphStyle {
        ParagraphStyle {
            id: "style_para_default".to_string(),
            name: "Default".to_string(),
            based_on: None,
            hidden: false,
            align: Some(ParagraphAlign::Center),
            left_indent_px: Some(0.0),
            right_indent_px: Some(0.0),
        }
    }

    #[test]
    fn resolve_fills_missing_fields_from_default() {
        let input = ParagraphStyle::new("style_para_input");
        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_para_input");
        assert_eq!(result.align, Some(ParagraphAlign::Center));
        assert_eq!(result.left_indent_px, Some(0.0));
        assert_eq!(result.right_indent_px, Some(0.0));
    }

    #[test]
    fn resolve_input_overrides_default() {
        let mut input = ParagraphStyle::new("style_para_start");
        input.align = Some(ParagraphAlign::Start);
        input.left_indent_px = Some(20.0);

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.align, Some(ParagraphAlign::Start));
        assert_eq!(result.left_indent_px, Some(20.0));
        assert_eq!(result.right_indent_px, Some(0.0));
    }

    #[test]
    fn resolve_follows_based_on_chain() {
        let default = default_style();

        let mut base = ParagraphStyle::new("style_para_base");
        base.align = Some(ParagraphAlign::Start);
        base.left_indent_px = Some(10.0);

        let mut derived = ParagraphStyle::new("style_para_derived");
        derived.based_on = Some("style_para_base".to_string());
        derived.align = Some(ParagraphAlign::End);

        let list = vec![base, derived];
        let result = resolve(&list[1], &list, &default).expect("valid table");

        assert_eq!(result.id, "style_para_derived");
        assert_eq!(result.align, Some(ParagraphAlign::End));
        assert_eq!(result.left_indent_px, Some(10.0));
        assert_eq!(result.right_indent_px, Some(0.0));
    }

    #[test]
    fn resolve_missing_based_on_uses_default() {
        let mut input = ParagraphStyle::new("style_para_input");
        input.based_on = Some("style_para_missing".to_string());

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_para_input");
        assert_eq!(result.align, Some(ParagraphAlign::Center));
    }

    #[test]
    fn resolve_default_incomplete_errors() {
        let mut incomplete = default_style();
        incomplete.align = None;

        let err = resolve(&ParagraphStyle::new("style_para_input"), &[], &incomplete)
            .expect_err("incomplete default must fail");
        assert_eq!(err.code, codes::DEFAULT_INCOMPLETE);
    }

    #[test]
    fn resolve_cycle_errors() {
        let mut a = ParagraphStyle::new("style_para_a");
        a.based_on = Some("style_para_b".to_string());

        let mut b = ParagraphStyle::new("style_para_b");
        b.based_on = Some("style_para_a".to_string());

        let list = vec![a, b];
        let err = resolve(&list[0], &list, &default_style()).expect_err("cycle must fail");
        assert_eq!(err.code, codes::BASED_ON_CYCLE);
    }
}
