//! Character style: typography, dual-state fill, wipe, and static
//! outline/shadow decorations for sung lyric characters.
//!
//! A [`CharacterStyle`] is both a named style entry and a sparse overlay:
//! optional fields are absent when they should be inherited from `based_on`
//! (or from the built-in default). [`resolve`] walks the `based_on` chain and
//! returns a concrete character style whose required fields are filled in.

use necokara_error::CkError;

use crate::fill::DualFill;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// The built-in default style has missing required fields.
    pub const DEFAULT_INCOMPLETE: &str =
        ck_code!("necokara-style", character, default_incomplete);
    /// The `based_on` chain contains a cycle.
    pub const BASED_ON_CYCLE: &str = ck_code!("necokara-style", character, based_on_cycle);
}

/// Karaoke wipe direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WipeDirection {
    /// Highlight sweeps left to right.
    Ltr,
    /// Highlight sweeps right to left.
    Rtl,
    /// Highlight sweeps top to bottom.
    Ttb,
    /// Highlight sweeps bottom to top.
    Btt,
    /// No directional wipe is applied.
    None,
}

/// Karaoke wipe transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WipeTransition {
    /// Uniform/linear progress.
    Linear,
    /// Smooth eased progress.
    Smooth,
}

/// Karaoke wipe behavior attached to a character style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Wipe {
    /// Sweep direction.
    pub direction: WipeDirection,
    /// Progress transition.
    pub transition: WipeTransition,
}

impl Default for Wipe {
    fn default() -> Self {
        Self {
            direction: WipeDirection::Ltr,
            transition: WipeTransition::Linear,
        }
    }
}

/// Static outline decoration. Its fill is dual-state so the outline color can
/// also switch between unsung and sung states.
#[derive(Debug, Clone, PartialEq)]
pub struct CkOutline {
    /// Unsung/sung outline colors; `active: None` means same as `normal`.
    pub fill: DualFill,
    /// Outline stroke width in pixels.
    pub width_px: f64,
    /// Outline blur radius in pixels.
    pub blur_px: f64,
}

/// Static shadow decoration. Its fill is dual-state like the main character
/// fill and the outline fill.
#[derive(Debug, Clone, PartialEq)]
pub struct CkShadow {
    /// Unsung/sung shadow colors; `active: None` means same as `normal`.
    pub fill: DualFill,
    /// Horizontal shadow offset in pixels.
    pub offset_x_px: f64,
    /// Vertical shadow offset in pixels.
    pub offset_y_px: f64,
    /// Shadow blur radius in pixels.
    pub blur_px: f64,
}

/// A named character style entry.
///
/// Optional visual fields follow the inheritance rule: `None` means "not
/// explicitly set here; inherit from `based_on` or from the built-in default".
/// `outline` and `shadow` keep their optional meaning after resolution: `None`
/// means that particular decoration is absent.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterStyle {
    /// Unique style id within the character style table.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Parent style id; `None` means the built-in default.
    pub based_on: Option<String>,
    /// Hidden styles are anonymous/direct-format entries, not shown in the UI.
    pub hidden: bool,

    // Typography and layout.
    /// CJK font fallback stack.
    pub cjk_families: Option<Vec<String>>,
    /// Western/Latin font fallback stack.
    pub western_families: Option<Vec<String>>,
    /// Font size in absolute pixels.
    pub font_size_px: Option<f64>,
    /// Font weight in the `100..=900` scale.
    pub font_weight: Option<u16>,
    /// Italic flag.
    pub italic: Option<bool>,
    /// Extra character spacing in pixels.
    pub letter_spacing_px: Option<f64>,
    /// Baseline shift in pixels.
    pub baseline_shift_px: Option<f64>,

    // Karaoke visual state.
    /// Dual-state character fill.
    pub fill: Option<DualFill>,
    /// Wipe behavior.
    pub wipe: Option<Wipe>,

    // Static decorations. `None` after resolution means "no outline/shadow".
    /// Optional outline.
    pub outline: Option<CkOutline>,
    /// Optional shadow.
    pub shadow: Option<CkShadow>,
}

impl CharacterStyle {
    /// Build an empty style with the given id and no explicit visual fields.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            based_on: None,
            hidden: false,
            cjk_families: None,
            western_families: None,
            font_size_px: None,
            font_weight: None,
            italic: None,
            letter_spacing_px: None,
            baseline_shift_px: None,
            fill: None,
            wipe: None,
            outline: None,
            shadow: None,
        }
    }
}

/// Resolve `input` against the character style table and the built-in default.
///
/// The merge order is:
///
/// ```text
/// built-in default
///   <- root ancestor in the based_on chain
///   <- ... parent ...
///   <- input style
/// ```
///
/// Returns an error when:
/// - the default style is missing any required field (`outline`/`shadow` may
///   legitimately be `None`);
/// - the `based_on` chain contains a cycle.
///
/// A missing `based_on` target is treated as the built-in default and is not
/// an error.
pub fn resolve(
    input: &CharacterStyle,
    list: &[CharacterStyle],
    default: &CharacterStyle,
) -> Result<CharacterStyle, CkError> {
    validate_default(default)?;

    // Collect the chain from input up to the root ancestor. The chain is later
    // applied in reverse so closest-to-default ancestors win first and input
    // wins last.
    let mut chain: Vec<&CharacterStyle> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut current = input;

    loop {
        if seen.contains(&current.id.as_str()) {
            return Err(CkError::new(
                codes::BASED_ON_CYCLE,
                format!("based_on cycle detected at style '{}'", current.id),
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
        apply_overrides(style, &mut result);
    }

    result.id = input.id.clone();
    result.name = input.name.clone();
    result.based_on = input.based_on.clone();
    result.hidden = input.hidden;
    Ok(result)
}

/// Ensure the built-in default is complete enough to resolve against.
///
/// Only the always-present resolved fields are required. `outline`/`shadow`
/// may be `None`, meaning the decoration is absent; `DualFill.active` may be
/// `None`, meaning the active state equals the normal state.
fn validate_default(default: &CharacterStyle) -> Result<(), CkError> {
    let mut missing: Vec<&'static str> = Vec::new();

    if default.cjk_families.is_none() {
        missing.push("cjk_families");
    }
    if default.western_families.is_none() {
        missing.push("western_families");
    }
    if default.font_size_px.is_none() {
        missing.push("font_size_px");
    }
    if default.font_weight.is_none() {
        missing.push("font_weight");
    }
    if default.italic.is_none() {
        missing.push("italic");
    }
    if default.letter_spacing_px.is_none() {
        missing.push("letter_spacing_px");
    }
    if default.baseline_shift_px.is_none() {
        missing.push("baseline_shift_px");
    }
    if default.fill.is_none() {
        missing.push("fill");
    }
    if default.wipe.is_none() {
        missing.push("wipe");
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(CkError::new(
            codes::DEFAULT_INCOMPLETE,
            format!("default character style is missing required fields: {}", missing.join(", ")),
        ))
    }
}

/// Apply one style's explicit (`Some`) fields onto the resolved result.
fn apply_overrides(style: &CharacterStyle, target: &mut CharacterStyle) {
    merge_opt(&mut target.cjk_families, &style.cjk_families);
    merge_opt(&mut target.western_families, &style.western_families);
    merge_opt(&mut target.font_size_px, &style.font_size_px);
    merge_opt(&mut target.font_weight, &style.font_weight);
    merge_opt(&mut target.italic, &style.italic);
    merge_opt(&mut target.letter_spacing_px, &style.letter_spacing_px);
    merge_opt(&mut target.baseline_shift_px, &style.baseline_shift_px);
    merge_opt(&mut target.fill, &style.fill);
    merge_opt(&mut target.wipe, &style.wipe);
    merge_opt(&mut target.outline, &style.outline);
    merge_opt(&mut target.shadow, &style.shadow);
}

/// Copy `source` into `target` only when `source` is explicitly set.
fn merge_opt<T: Clone>(target: &mut Option<T>, source: &Option<T>) {
    if let Some(value) = source {
        *target = Some(value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::CkColor;
    use crate::fill::{DualFill, Fill};

    fn solid(color: CkColor) -> Fill {
        Fill::Solid { color }
    }

    fn default_style() -> CharacterStyle {
        CharacterStyle {
            id: "style_char_default".to_string(),
            name: "Default".to_string(),
            based_on: None,
            hidden: false,
            cjk_families: Some(vec!["Yu Gothic UI".to_string()]),
            western_families: Some(vec!["Segoe UI".to_string()]),
            font_size_px: Some(72.0),
            font_weight: Some(400),
            italic: Some(false),
            letter_spacing_px: Some(0.0),
            baseline_shift_px: Some(0.0),
            fill: Some(DualFill::new(
                solid(CkColor::rgb(255, 255, 255)),
                Some(solid(CkColor::rgb(255, 230, 0))),
            )),
            wipe: Some(Wipe::default()),
            outline: None,
            shadow: None,
        }
    }

    #[test]
    fn resolve_fills_missing_fields_from_default() {
        let input = CharacterStyle::new("style_char_input");
        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_char_input");
        assert_eq!(result.font_size_px, Some(72.0));
        assert_eq!(result.font_weight, Some(400));
        assert_eq!(result.italic, Some(false));
        assert!(result.fill.is_some());
        assert_eq!(result.outline, None);
        assert_eq!(result.shadow, None);
    }

    #[test]
    fn resolve_input_overrides_default() {
        let mut input = CharacterStyle::new("style_char_big");
        input.font_size_px = Some(120.0);
        input.font_weight = Some(700);
        input.italic = Some(true);

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.font_size_px, Some(120.0));
        assert_eq!(result.font_weight, Some(700));
        assert_eq!(result.italic, Some(true));
        // Fields not overridden still come from the default.
        assert!(result.fill.is_some());
        assert_eq!(result.cjk_families, Some(vec!["Yu Gothic UI".to_string()]));
    }

    #[test]
    fn resolve_follows_based_on_chain() {
        let default = default_style();

        let mut base = CharacterStyle::new("style_char_base");
        base.font_size_px = Some(60.0);
        base.fill = Some(DualFill::new(
            solid(CkColor::rgb(0, 0, 0)),
            Some(solid(CkColor::rgb(255, 255, 255))),
        ));

        let mut derived = CharacterStyle::new("style_char_derived");
        derived.based_on = Some("style_char_base".to_string());
        derived.font_size_px = Some(80.0);

        let list = vec![base, derived];
        let result = resolve(&list[1], &list, &default).expect("valid style table");

        assert_eq!(result.id, "style_char_derived");
        assert_eq!(result.font_size_px, Some(80.0));
        // Base fill survived because derived did not override it.
        assert_eq!(
            result.fill,
            Some(DualFill::new(
                solid(CkColor::rgb(0, 0, 0)),
                Some(solid(CkColor::rgb(255, 255, 255))),
            ))
        );
    }

    #[test]
    fn resolve_missing_based_on_uses_default() {
        let mut input = CharacterStyle::new("style_char_input");
        input.based_on = Some("style_char_missing".to_string());

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        assert_eq!(result.id, "style_char_input");
        assert_eq!(result.font_size_px, Some(72.0));
    }

    #[test]
    fn resolve_default_incomplete_errors() {
        let mut incomplete = default_style();
        incomplete.font_size_px = None;
        incomplete.fill = None;

        let err = resolve(&CharacterStyle::new("style_char_input"), &[], &incomplete)
            .expect_err("incomplete default must fail");
        assert_eq!(err.code, codes::DEFAULT_INCOMPLETE);
    }

    #[test]
    fn resolve_cycle_errors() {
        let mut a = CharacterStyle::new("style_char_a");
        a.based_on = Some("style_char_b".to_string());
        a.font_size_px = Some(10.0);

        let mut b = CharacterStyle::new("style_char_b");
        b.based_on = Some("style_char_a".to_string());
        b.font_size_px = Some(20.0);

        let list = vec![a, b];
        let err = resolve(&list[0], &list, &default_style()).expect_err("cycle must fail");
        assert_eq!(err.code, codes::BASED_ON_CYCLE);
    }

    #[test]
    fn self_based_on_errors() {
        let mut input = CharacterStyle::new("style_char_self");
        input.based_on = Some("style_char_self".to_string());

        let list = vec![input.clone()];
        let err = resolve(&input, &list, &default_style()).expect_err("self cycle must fail");
        assert_eq!(err.code, codes::BASED_ON_CYCLE);
    }

    #[test]
    fn outline_and_shadow_merge_as_whole_values() {
        let default = default_style();

        let mut input = CharacterStyle::new("style_char_outlined");
        input.outline = Some(CkOutline {
            fill: DualFill::new(
                solid(CkColor::rgb(0, 0, 0)),
                Some(solid(CkColor::rgb(255, 0, 0))),
            ),
            width_px: 4.0,
            blur_px: 0.0,
        });
        input.shadow = Some(CkShadow {
            fill: DualFill::new(
                solid(CkColor::rgb(0, 0, 0)),
                Some(solid(CkColor::rgb(0, 0, 0))),
            ),
            offset_x_px: 2.0,
            offset_y_px: 3.0,
            blur_px: 1.0,
        });

        let result = resolve(&input, &[], &default).expect("valid default");

        assert!(result.outline.is_some());
        assert!(result.shadow.is_some());
        assert_eq!(result.outline.as_ref().unwrap().width_px, 4.0);
    }

    #[test]
    fn dual_fill_active_none_means_same_as_normal() {
        let mut input = CharacterStyle::new("style_char_mono");
        input.fill = Some(DualFill::mono(solid(CkColor::rgb(255, 255, 255))));

        let result = resolve(&input, &[], &default_style()).expect("valid default");

        let fill = result.fill.expect("fill resolved");
        assert_eq!(fill.active, None);
    }
}
