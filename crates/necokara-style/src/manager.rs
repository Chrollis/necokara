//! Style manager: definitions (`StyleBook`) + applications (`StyleAlloc`) +
//! built-in defaults, with convenience resolution queries.

use necokara_error::CkError;

use crate::alloc::StyleAlloc;
use crate::book::StyleBook;
use crate::character::{resolve as resolve_character, CharacterStyle};
use crate::page::{resolve as resolve_page, PageStyle};
use crate::paragraph::{resolve as resolve_paragraph, ParagraphStyle};
use crate::ruby::{resolve as resolve_ruby, RubyStyle};

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// A style id referenced by `StyleAlloc` is not present in `StyleBook`.
    pub const STYLE_NOT_FOUND: &str = ck_code!("necokara-style", manager, style_not_found);
}

/// Combined style definitions, defaults, and applications.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleManager {
    /// Named style tables.
    pub book: StyleBook,
    /// Built-in default character style.
    pub default_character_style: CharacterStyle,
    /// Built-in default paragraph style.
    pub default_paragraph_style: ParagraphStyle,
    /// Built-in default page style.
    pub default_page_style: PageStyle,
    /// Built-in default ruby style.
    pub default_ruby_style: RubyStyle,
    /// Style applications over the lyric/document index axes.
    pub alloc: StyleAlloc,
}

impl StyleManager {
    /// Build a style manager from definitions, defaults, and applications.
    pub fn new(
        book: StyleBook,
        default_character_style: CharacterStyle,
        default_paragraph_style: ParagraphStyle,
        default_page_style: PageStyle,
        default_ruby_style: RubyStyle,
        alloc: StyleAlloc,
    ) -> Self {
        Self {
            book,
            default_character_style,
            default_paragraph_style,
            default_page_style,
            default_ruby_style,
            alloc,
        }
    }

    /// Resolve the effective character style at a main-stream index.
    pub fn resolve_main_char_at(&self, index: usize) -> Result<CharacterStyle, CkError> {
        match self.alloc.main_char_style_id_at(index) {
            None => Ok(self.default_character_style.clone()),
            Some(id) => {
                let style = self
                    .book
                    .character_style(id)
                    .ok_or_else(|| style_missing("character", id))?;
                resolve_character(style, &self.book.character_styles, &self.default_character_style)
            }
        }
    }

    /// Resolve the effective character style at a ruby-stream index.
    pub fn resolve_ruby_char_at(&self, index: usize) -> Result<CharacterStyle, CkError> {
        match self.alloc.ruby_char_style_id_at(index) {
            None => Ok(self.default_character_style.clone()),
            Some(id) => {
                let style = self
                    .book
                    .character_style(id)
                    .ok_or_else(|| style_missing("character", id))?;
                resolve_character(style, &self.book.character_styles, &self.default_character_style)
            }
        }
    }

    /// Resolve the effective ruby layout style at a word index.
    pub fn resolve_word_ruby_at(&self, word_index: usize) -> Result<RubyStyle, CkError> {
        match self.alloc.word_ruby_style_id_at(word_index) {
            None => Ok(self.default_ruby_style.clone()),
            Some(id) => {
                let style = self
                    .book
                    .ruby_style(id)
                    .ok_or_else(|| style_missing("ruby", id))?;
                resolve_ruby(style, &self.book.ruby_styles, &self.default_ruby_style)
            }
        }
    }

    /// Resolve the effective paragraph style at a line index.
    pub fn resolve_line_paragraph_at(&self, line_index: usize) -> Result<ParagraphStyle, CkError> {
        match self.alloc.line_paragraph_style_id_at(line_index) {
            None => Ok(self.default_paragraph_style.clone()),
            Some(id) => {
                let style = self
                    .book
                    .paragraph_style(id)
                    .ok_or_else(|| style_missing("paragraph", id))?;
                resolve_paragraph(
                    style,
                    &self.book.paragraph_styles,
                    &self.default_paragraph_style,
                )
            }
        }
    }

    /// Resolve the effective page style at a line index.
    pub fn resolve_line_page_at(&self, line_index: usize) -> Result<PageStyle, CkError> {
        match self.alloc.page_style_id_at(line_index) {
            None => Ok(self.default_page_style.clone()),
            Some(id) => {
                let style = self
                    .book
                    .page_style(id)
                    .ok_or_else(|| style_missing("page", id))?;
                resolve_page(style, &self.book.page_styles, &self.default_page_style)
            }
        }
    }
}

/// Build the error for a dangling style id.
fn style_missing(kind: &str, id: &str) -> CkError {
    CkError::new(
        codes::STYLE_NOT_FOUND,
        format!("{kind} style not found in style book: {id}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::CkColor;
    use crate::fill::{DualFill, Fill};
    use crate::page::{PageMargins, PageTextDirection, VerticalAlign};
    use crate::paragraph::ParagraphAlign;
    use crate::ruby::{RubyAlign, RubyPosition};

    fn solid() -> Fill {
        Fill::Solid {
            color: CkColor::rgb(255, 255, 255),
        }
    }

    fn default_character() -> CharacterStyle {
        let mut style = CharacterStyle::new("style_char_default");
        style.cjk_families = Some(vec!["Yu Gothic UI".to_string()]);
        style.western_families = Some(vec!["Segoe UI".to_string()]);
        style.font_size_px = Some(72.0);
        style.font_weight = Some(400);
        style.italic = Some(false);
        style.letter_spacing_px = Some(0.0);
        style.baseline_shift_px = Some(0.0);
        style.fill = Some(DualFill::new(solid(), None));
        style.wipe = Some(crate::character::Wipe::default());
        style
    }

    fn default_paragraph() -> ParagraphStyle {
        let mut style = ParagraphStyle::new("style_para_default");
        style.align = Some(ParagraphAlign::Center);
        style.left_indent_px = Some(0.0);
        style.right_indent_px = Some(0.0);
        style
    }

    fn default_page() -> PageStyle {
        let mut style = PageStyle::new("style_page_default");
        style.text_direction = Some(PageTextDirection::Horizontal);
        style.margins = Some(PageMargins::new(80.0, 80.0, 80.0, 80.0));
        style.align = Some(VerticalAlign::Middle);
        style.line_spacing_px = Some(0.0);
        style
    }

    fn default_ruby() -> RubyStyle {
        let mut style = RubyStyle::new("style_ruby_default");
        style.position = Some(RubyPosition::Above);
        style.align = Some(RubyAlign::Center);
        style.distance_px = Some(6.0);
        style
    }

    fn manager() -> StyleManager {
        StyleManager::new(
            StyleBook::new(),
            default_character(),
            default_paragraph(),
            default_page(),
            default_ruby(),
            StyleAlloc::new(),
        )
    }

    #[test]
    fn no_application_returns_default() {
        let manager = manager();

        assert_eq!(
            manager.resolve_main_char_at(0).unwrap().id,
            "style_char_default"
        );
        assert_eq!(
            manager.resolve_line_paragraph_at(0).unwrap().id,
            "style_para_default"
        );
        assert_eq!(
            manager.resolve_line_page_at(0).unwrap().id,
            "style_page_default"
        );
        assert_eq!(
            manager.resolve_word_ruby_at(0).unwrap().id,
            "style_ruby_default"
        );
    }

    #[test]
    fn applied_style_is_resolved() {
        let mut book = StyleBook::new();
        let mut style = CharacterStyle::new("style_char_big");
        style.font_size_px = Some(120.0);
        book.character_styles.push(style);

        let mut alloc = StyleAlloc::new();
        alloc.set_main_char_style(0, 1, "style_char_big");

        let manager = StyleManager::new(
            book,
            default_character(),
            default_paragraph(),
            default_page(),
            default_ruby(),
            alloc,
        );

        let resolved = manager.resolve_main_char_at(0).unwrap();
        assert_eq!(resolved.id, "style_char_big");
        assert_eq!(resolved.font_size_px, Some(120.0));
        // Fields not specified by the applied style inherit the default.
        assert_eq!(resolved.font_weight, Some(400));
    }

    #[test]
    fn dangling_style_id_errors() {
        let mut alloc = StyleAlloc::new();
        alloc.set_line_paragraph_style(0, 1, "style_para_missing");

        let manager = StyleManager::new(
            StyleBook::new(),
            default_character(),
            default_paragraph(),
            default_page(),
            default_ruby(),
            alloc,
        );

        let err = manager.resolve_line_paragraph_at(0).unwrap_err();
        assert_eq!(err.code, codes::STYLE_NOT_FOUND);
    }
}
