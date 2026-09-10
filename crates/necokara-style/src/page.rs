//! Page style: layout arrangement for a page / subtitle text box.
//!
//! A page behaves like a text box: it has a text direction, margins, vertical
//! alignment, line spacing, and optional per-line extra offsets. Page styles
//! do not contain canvas width/height; those belong to project settings.

use necokara_error::CkError;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// The built-in default page style has missing required fields.
    pub const DEFAULT_INCOMPLETE: &str = ck_code!("necokara-style", page, default_incomplete);
    /// The `based_on` chain contains a cycle.
    pub const BASED_ON_CYCLE: &str = ck_code!("necokara-style", page, based_on_cycle);
}

/// Text direction inside a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageTextDirection {
    /// Horizontal text flow (left to right / top to bottom rows).
    Horizontal,
    /// Vertical text flow (top to bottom columns).
    Vertical,
    /// Horizontal text rotated 90 degrees.
    Rotate90,
    /// Horizontal text rotated 270 degrees.
    Rotate270,
}

/// Vertical alignment of the whole text block inside the page area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerticalAlign {
    /// Anchor the text block near the top.
    Top,
    /// Center the text block vertically.
    Middle,
    /// Anchor the text block near the bottom.
    Bottom,
}

/// Page margins in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageMargins {
    /// Top margin in pixels.
    pub top_px: f64,
    /// Right margin in pixels.
    pub right_px: f64,
    /// Bottom margin in pixels.
    pub bottom_px: f64,
    /// Left margin in pixels.
    pub left_px: f64,
}

impl PageMargins {
    /// Build margins from four pixel values.
    pub fn new(top_px: f64, right_px: f64, bottom_px: f64, left_px: f64) -> Self {
        Self {
            top_px,
            right_px,
            bottom_px,
            left_px,
        }
    }
}

/// Extra offset applied to one line after the normal text-box layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineOffset {
    /// Extra horizontal offset in pixels.
    pub offset_x_px: f64,
    /// Extra vertical offset in pixels.
    pub offset_y_px: f64,
}

impl LineOffset {
    /// Build a per-line extra offset.
    pub fn new(offset_x_px: f64, offset_y_px: f64) -> Self {
        Self {
            offset_x_px,
            offset_y_px,
        }
    }
}

/// A named page / text-box layout style entry.
///
/// Optional fields follow the same inheritance convention as the other style
/// kinds: `None` means "inherit from `based_on` or from the built-in default".
/// `line_offsets: None` means no per-line overrides are applied.
#[derive(Debug, Clone, PartialEq)]
pub struct PageStyle {
    /// Unique style id within the page style table.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Parent style id; `None` means the built-in default.
    pub based_on: Option<String>,
    /// Hidden styles are anonymous/direct-format entries, not shown in the UI.
    pub hidden: bool,

    /// Text direction used by this page.
    pub text_direction: Option<PageTextDirection>,
    /// Page margins in pixels.
    pub margins: Option<PageMargins>,
    /// Vertical alignment of the text block.
    pub align: Option<VerticalAlign>,
    /// Extra spacing between lines in pixels.
    pub line_spacing_px: Option<f64>,
    /// Optional per-line extra offsets (index in the vector = page line order).
    pub line_offsets: Option<Vec<LineOffset>>,
}

impl PageStyle {
    /// Build an empty page style with the given id and no explicit fields.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            based_on: None,
            hidden: false,
            text_direction: None,
            margins: None,
            align: None,
            line_spacing_px: None,
            line_offsets: None,
        }
    }
}

/// Resolve a page style against the page style table and the built-in default.
///
/// Returns an error when the default is missing a required field or when the
/// `based_on` chain contains a cycle. `line_offsets` may be `None`, meaning no
/// per-line offsets.
pub fn resolve(
    input: &PageStyle,
    list: &[PageStyle],
    default: &PageStyle,
) -> Result<PageStyle, CkError> {
    validate_default(default)?;

    let mut chain: Vec<&PageStyle> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut current = input;

    loop {
        if seen.contains(&current.id.as_str()) {
            return Err(CkError::new(
                codes::BASED_ON_CYCLE,
                format!("based_on cycle detected at page style '{}'", current.id),
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
        if let Some(text_direction) = style.text_direction {
            result.text_direction = Some(text_direction);
        }
        if let Some(margins) = style.margins {
            result.margins = Some(margins);
        }
        if let Some(align) = style.align {
            result.align = Some(align);
        }
        if let Some(line_spacing_px) = style.line_spacing_px {
            result.line_spacing_px = Some(line_spacing_px);
        }
        if let Some(line_offsets) = &style.line_offsets {
            result.line_offsets = Some(line_offsets.clone());
        }
    }

    result.id = input.id.clone();
    result.name = input.name.clone();
    result.based_on = input.based_on.clone();
    result.hidden = input.hidden;
    Ok(result)
}

/// Ensure the built-in default contains all required page layout values.
fn validate_default(default: &PageStyle) -> Result<(), CkError> {
    let mut missing: Vec<&'static str> = Vec::new();

    if default.text_direction.is_none() {
        missing.push("text_direction");
    }
    if default.margins.is_none() {
        missing.push("margins");
    }
    if default.align.is_none() {
        missing.push("align");
    }
    if default.line_spacing_px.is_none() {
        missing.push("line_spacing_px");
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(CkError::new(
            codes::DEFAULT_INCOMPLETE,
            format!(
                "default page style is missing required fields: {}",
                missing.join(", ")
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_style() -> PageStyle {
        PageStyle {
            id: "style_page_default".to_string(),
            name: "Default".to_string(),
            based_on: None,
            hidden: false,
            text_direction: Some(PageTextDirection::Horizontal),
            margins: Some(PageMargins::new(80.0, 80.0, 80.0, 80.0)),
            align: Some(VerticalAlign::Middle),
            line_spacing_px: Some(0.0),
            line_offsets: None,
        }
    }

    #[test]
    fn resolve_fills_missing_fields_from_default() {
        let input = PageStyle::new("style_page_input");
        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_page_input");
        assert_eq!(result.text_direction, Some(PageTextDirection::Horizontal));
        assert_eq!(result.align, Some(VerticalAlign::Middle));
        assert_eq!(result.margins, Some(PageMargins::new(80.0, 80.0, 80.0, 80.0)));
        assert_eq!(result.line_spacing_px, Some(0.0));
        assert_eq!(result.line_offsets, None);
    }

    #[test]
    fn resolve_input_overrides_default() {
        let mut input = PageStyle::new("style_page_bottom");
        input.align = Some(VerticalAlign::Bottom);
        input.text_direction = Some(PageTextDirection::Vertical);
        input.line_offsets = Some(vec![
            LineOffset::new(0.0, 12.0),
            LineOffset::new(20.0, 0.0),
        ]);

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.align, Some(VerticalAlign::Bottom));
        assert_eq!(result.text_direction, Some(PageTextDirection::Vertical));
        assert_eq!(
            result.line_offsets,
            Some(vec![LineOffset::new(0.0, 12.0), LineOffset::new(20.0, 0.0)])
        );
        assert_eq!(result.margins, Some(PageMargins::new(80.0, 80.0, 80.0, 80.0)));
    }

    #[test]
    fn resolve_follows_based_on_chain() {
        let default = default_style();

        let mut base = PageStyle::new("style_page_base");
        base.margins = Some(PageMargins::new(40.0, 60.0, 40.0, 60.0));
        base.align = Some(VerticalAlign::Top);

        let mut derived = PageStyle::new("style_page_derived");
        derived.based_on = Some("style_page_base".to_string());
        derived.align = Some(VerticalAlign::Bottom);

        let list = vec![base, derived];
        let result = resolve(&list[1], &list, &default).expect("valid table");

        assert_eq!(result.id, "style_page_derived");
        assert_eq!(result.align, Some(VerticalAlign::Bottom));
        assert_eq!(result.margins, Some(PageMargins::new(40.0, 60.0, 40.0, 60.0)));
    }

    #[test]
    fn resolve_missing_based_on_uses_default() {
        let mut input = PageStyle::new("style_page_input");
        input.based_on = Some("style_page_missing".to_string());

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_page_input");
        assert_eq!(result.align, Some(VerticalAlign::Middle));
    }

    #[test]
    fn resolve_default_incomplete_errors() {
        let mut incomplete = default_style();
        incomplete.margins = None;
        incomplete.line_spacing_px = None;

        let err = resolve(&PageStyle::new("style_page_input"), &[], &incomplete)
            .expect_err("incomplete default must fail");
        assert_eq!(err.code, codes::DEFAULT_INCOMPLETE);
    }

    #[test]
    fn resolve_cycle_errors() {
        let mut a = PageStyle::new("style_page_a");
        a.based_on = Some("style_page_b".to_string());

        let mut b = PageStyle::new("style_page_b");
        b.based_on = Some("style_page_a".to_string());

        let list = vec![a, b];
        let err = resolve(&list[0], &list, &default_style()).expect_err("cycle must fail");
        assert_eq!(err.code, codes::BASED_ON_CYCLE);
    }
}
