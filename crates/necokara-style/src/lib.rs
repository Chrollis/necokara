//! Necokara style definitions, applications, and inheritance resolution.
//!
//! This crate is the style layer of the core data model. It provides:
//!
//! - style definitions: character, ruby layout, page layout, paragraph (line);
//! - [`StyleBook`], the four named-style tables;
//! - [`StyleAlloc`], style application runs over lyric/document index axes;
//! - [`StyleManager`], definitions + defaults + applications with resolved
//!   lookup queries.
//!
//! Character styles describe the visual look of lyric characters themselves:
//! typography, the dual-state karaoke fill, and static outline/shadow
//! decorations. Decorative text formatting that is not part of the sung lyric
//! (underline, strikethrough, small caps, superscript, glow, animation, etc.)
//! is intentionally out of scope.

pub mod alloc;
pub mod book;
pub mod character;
pub mod color;
pub mod fill;
pub mod manager;
pub mod page;
pub mod paragraph;
pub mod ruby;

pub use alloc::{StyleAlloc, StyleRun};
pub use book::StyleBook;
pub use character::{
    resolve, CharacterStyle, CkOutline, CkShadow, Wipe, WipeDirection, WipeTransition,
};
pub use color::CkColor;
pub use fill::{DualFill, Fill, FillStop};
pub use manager::StyleManager;
pub use page::resolve as resolve_page_style;
pub use page::{LineOffset, PageMargins, PageStyle, PageTextDirection, VerticalAlign};
pub use paragraph::resolve as resolve_paragraph_style;
pub use paragraph::{ParagraphAlign, ParagraphStyle};
pub use ruby::resolve as resolve_ruby_style;
pub use ruby::{RubyAlign, RubyPosition, RubyStyle};
