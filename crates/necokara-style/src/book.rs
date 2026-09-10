//! Style book: the four named style tables.
//!
//! `StyleBook` owns style definitions only. Where a style is applied lives in
//! [`StyleAlloc`](crate::StyleAlloc); resolving a definition against its
//! `based_on` chain lives in the per-kind `resolve_*` functions or in
//! [`StyleManager`](crate::StyleManager).

use crate::character::CharacterStyle;
use crate::page::PageStyle;
use crate::paragraph::ParagraphStyle;
use crate::ruby::RubyStyle;

/// The named style tables for every style kind.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleBook {
    /// Character style definitions.
    pub character_styles: Vec<CharacterStyle>,
    /// Paragraph (line) style definitions.
    pub paragraph_styles: Vec<ParagraphStyle>,
    /// Page layout style definitions.
    pub page_styles: Vec<PageStyle>,
    /// Ruby layout style definitions.
    pub ruby_styles: Vec<RubyStyle>,
}

impl StyleBook {
    /// Build an empty style book.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a character style by id.
    pub fn character_style(&self, id: &str) -> Option<&CharacterStyle> {
        self.character_styles.iter().find(|style| style.id == id)
    }

    /// Look up a paragraph style by id.
    pub fn paragraph_style(&self, id: &str) -> Option<&ParagraphStyle> {
        self.paragraph_styles.iter().find(|style| style.id == id)
    }

    /// Look up a page style by id.
    pub fn page_style(&self, id: &str) -> Option<&PageStyle> {
        self.page_styles.iter().find(|style| style.id == id)
    }

    /// Look up a ruby style by id.
    pub fn ruby_style(&self, id: &str) -> Option<&RubyStyle> {
        self.ruby_styles.iter().find(|style| style.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_book_has_no_styles() {
        let book = StyleBook::new();
        assert!(book.character_style("style_char_0").is_none());
        assert!(book.paragraph_style("style_para_0").is_none());
        assert!(book.page_style("style_page_0").is_none());
        assert!(book.ruby_style("style_ruby_0").is_none());
    }

    #[test]
    fn lookup_by_id() {
        let book = StyleBook {
            character_styles: vec![CharacterStyle::new("style_char_0")],
            paragraph_styles: vec![ParagraphStyle::new("style_para_0")],
            page_styles: vec![PageStyle::new("style_page_0")],
            ruby_styles: vec![RubyStyle::new("style_ruby_0")],
        };

        assert_eq!(
            book.character_style("style_char_0").map(|s| s.id.as_str()),
            Some("style_char_0")
        );
        assert_eq!(
            book.paragraph_style("style_para_0").map(|s| s.id.as_str()),
            Some("style_para_0")
        );
        assert_eq!(
            book.page_style("style_page_0").map(|s| s.id.as_str()),
            Some("style_page_0")
        );
        assert_eq!(
            book.ruby_style("style_ruby_0").map(|s| s.id.as_str()),
            Some("style_ruby_0")
        );
    }
}
