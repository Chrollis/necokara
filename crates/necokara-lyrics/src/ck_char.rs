//! Character-level classification for lyric text.
//!
//! `CkChar` wraps a `char` and provides:
//!  - the four lightweight split categories used by the default tokenizer
//!    ([`CharKind`]), and
//!  - thin wrappers over the underlying Unicode property crates
//!    (`unic-ucd-category`, `yeslogic-unicode-script`) so callers never need
//!    to depend on them directly.
//!
//! Naming follows the project-wide `ck_` prefix domain: types are named
//! `CkChar`, fields/functions snake_case.

use unic_ucd_category::GeneralCategory;
use unicode_script::{get_script, Script};

/// Split category of a character for default auto-tokenization.
///
/// Mirrors the four-way classification in the reference implementations:
/// ideographs/phonetic/symbols split per character, letters and digits per
/// word, whitespace separates, everything else is treated as asyllabic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharKind {
    /// Split per character: CJK ideographs, kana/hangul, symbols, numbers.
    PerChar,
    /// Split per word: letters and decimal digits.
    PerWord,
    /// Whitespace / separators (not a singing unit).
    Separator,
    /// Everything else (e.g. combining marks): no default split rule.
    Asyllabic,
}

/// A lyric character: wraps a Unicode scalar value with classification
/// queries. Cheap to copy; the underlying char is the single source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CkChar {
    ch: char,
}

impl CkChar {
    /// Wrap a `char`.
    pub fn new(ch: char) -> Self {
        Self { ch }
    }

    /// The wrapped scalar value.
    pub fn value(self) -> char {
        self.ch
    }

    /// Whether this is a newline (the line separator in the character stream).
    pub fn is_newline(self) -> bool {
        self.ch == '\n'
    }

    /// Whether this is a tab, CR/LF, or Unicode separator.
    pub fn is_whitespace(self) -> bool {
        matches!(self.ch, ' ' | '\t' | '\r' | '\n') || GeneralCategory::of(self.ch).is_separator()
    }

    /// Four-way split category used by the default tokenizer.
    pub fn kind(self) -> CharKind {
        let gc = GeneralCategory::of(self.ch);
        // Hard separators and Unicode separators never form a singing unit.
        if self.ch == '\n' || self.ch == '\r' || self.ch == '\t' || gc.is_separator() {
            return CharKind::Separator;
        }

        let sc = get_script(self.ch);
        // Ideographic / phonetic scripts: one character = one unit.
        if matches!(
            sc,
            Script::Han | Script::Hangul | Script::Bopomofo | Script::Katakana | Script::Hiragana
        ) {
            return CharKind::PerChar;
        }
        // Symbols and numeric characters (roman numerals etc.) also split per
        // character.
        if gc.is_symbol() || matches!(gc, GeneralCategory::LetterNumber | GeneralCategory::OtherNumber)
        {
            return CharKind::PerChar;
        }
        // Ordinary letters and decimal digits group per word.
        if gc.is_cased_letter()
            || matches!(
                gc,
                GeneralCategory::ModifierLetter
                    | GeneralCategory::OtherLetter
                    | GeneralCategory::DecimalNumber
            )
        {
            return CharKind::PerWord;
        }
        // Everything else (marks, punctuation handled elsewhere, controls鈥?.
        CharKind::Asyllabic
    }

    /// Whether `kind()` is [`CharKind::Separator`].
    pub fn is_separator(self) -> bool {
        self.kind() == CharKind::Separator
    }

    /// Whether `kind()` is [`CharKind::PerChar`].
    pub fn is_per_char(self) -> bool {
        self.kind() == CharKind::PerChar
    }

    /// Whether `kind()` is [`CharKind::PerWord`].
    pub fn is_per_word(self) -> bool {
        self.kind() == CharKind::PerWord
    }

    /// Whether `kind()` is [`CharKind::Asyllabic`].
    pub fn is_asyllabic(self) -> bool {
        self.kind() == CharKind::Asyllabic
    }

    // ---- Unicode property wrappers (unic-ucd-category) ----

    /// Unicode General_Category of the wrapped char.
    pub fn general_category(self) -> GeneralCategory {
        GeneralCategory::of(self.ch)
    }

    /// Letter (L*).
    pub fn is_letter(self) -> bool {
        GeneralCategory::of(self.ch).is_letter()
    }

    /// Cased letter (Lu|Ll|Lt).
    pub fn is_cased_letter(self) -> bool {
        GeneralCategory::of(self.ch).is_cased_letter()
    }

    /// Uppercase letter (Lu).
    pub fn is_uppercase(self) -> bool {
        GeneralCategory::of(self.ch) == GeneralCategory::UppercaseLetter
    }

    /// Lowercase letter (Ll).
    pub fn is_lowercase(self) -> bool {
        GeneralCategory::of(self.ch) == GeneralCategory::LowercaseLetter
    }

    /// Mark (M*), e.g. combining marks.
    pub fn is_mark(self) -> bool {
        GeneralCategory::of(self.ch).is_mark()
    }

    /// Number (N*).
    pub fn is_number(self) -> bool {
        GeneralCategory::of(self.ch).is_number()
    }

    /// Decimal digit (Nd).
    pub fn is_digit(self) -> bool {
        GeneralCategory::of(self.ch) == GeneralCategory::DecimalNumber
    }

    /// Punctuation (P*).
    pub fn is_punctuation(self) -> bool {
        GeneralCategory::of(self.ch).is_punctuation()
    }

    /// Symbol (S*).
    pub fn is_symbol(self) -> bool {
        GeneralCategory::of(self.ch).is_symbol()
    }

    /// Control/format/other (C*).
    pub fn is_other(self) -> bool {
        GeneralCategory::of(self.ch).is_other()
    }

    // ---- Unicode script wrapper (yeslogic-unicode-script) ----

    /// Unicode Script of the wrapped char.
    pub fn script(self) -> Script {
        get_script(self.ch)
    }

    /// Whether the char belongs to the Han (CJK ideograph) script.
    pub fn is_han(self) -> bool {
        self.script() == Script::Han
    }

    /// Whether the char belongs to a Japanese kana script (hiragana/katakana).
    pub fn is_kana(self) -> bool {
        matches!(self.script(), Script::Hiragana | Script::Katakana)
    }
}

impl From<char> for CkChar {
    fn from(ch: char) -> Self {
        Self::new(ch)
    }
}

impl From<CkChar> for char {
    fn from(c: CkChar) -> char {
        c.value()
    }
}

impl std::fmt::Display for CkChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_of(c: char) -> CharKind {
        CkChar::new(c).kind()
    }

    #[test]
    fn cjk_and_kana_are_per_char() {
        assert_eq!(kind_of('明'), CharKind::PerChar);
        assert_eq!(kind_of('あ'), CharKind::PerChar);
        assert_eq!(kind_of('カ'), CharKind::PerChar);
        assert_eq!(kind_of('한'), CharKind::PerChar);
        assert_eq!(kind_of('ㄅ'), CharKind::PerChar);
    }

    #[test]
    fn latin_and_digits_are_per_word() {
        assert_eq!(kind_of('a'), CharKind::PerWord);
        assert_eq!(kind_of('Z'), CharKind::PerWord);
        assert_eq!(kind_of('5'), CharKind::PerWord);
    }

    #[test]
    fn separators() {
        assert_eq!(kind_of(' '), CharKind::Separator);
        assert_eq!(kind_of('\n'), CharKind::Separator);
        assert_eq!(kind_of('\t'), CharKind::Separator);
    }

    #[test]
    fn symbols_per_char() {
        assert_eq!(kind_of('♪'), CharKind::PerChar);
    }

    #[test]
    fn helpers() {
        assert!(CkChar::new('\n').is_newline());
        assert!(CkChar::new('あ').is_kana());
        assert!(CkChar::new('明').is_han());
        assert!(!CkChar::new('a').is_han());
    }
}
