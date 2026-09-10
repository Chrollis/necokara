//! Time values and their parsing/formatting.
//!
//! `CkTime` is an `i64` count of milliseconds. Formatting/parsing follows the
//! C++ reference (`time_format`): a configurable decimal point, number of
//! decimal places (3 = ms, 2 = centiseconds), and bracket style. Standard
//! presets mirror LRC (`[mm:ss.mmm]`) and NicoKaraMaker3 (`[mm:ss:cc]`).
//!
//! Core stores times only as integer milliseconds; strings are exchanged at
//! the translation/IO layer via [`CkTime::parse`] and [`CkTime::format`].

use necokara_error::CkError;

/// A time value in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CkTime {
    pub ms: i64,
}

impl CkTime {
    /// Create from a millisecond count.
    pub const fn new(ms: i64) -> Self {
        Self { ms }
    }

    /// Zero time.
    pub const ZERO: Self = Self { ms: 0 };

    /// Whether this value is negative.
    pub fn is_negative(self) -> bool {
        self.ms < 0
    }

    /// Absolute value in milliseconds.
    pub fn abs_ms(self) -> i64 {
        self.ms.abs()
    }
}

impl From<i64> for CkTime {
    fn from(ms: i64) -> Self {
        Self::new(ms)
    }
}

impl From<CkTime> for i64 {
    fn from(t: CkTime) -> i64 {
        t.ms
    }
}

impl std::ops::Add for CkTime {
    type Output = CkTime;
    fn add(self, rhs: CkTime) -> CkTime {
        CkTime::new(self.ms + rhs.ms)
    }
}

impl std::ops::Sub for CkTime {
    type Output = CkTime;
    fn sub(self, rhs: CkTime) -> CkTime {
        CkTime::new(self.ms - rhs.ms)
    }
}

impl std::ops::Neg for CkTime {
    type Output = CkTime;
    fn neg(self) -> CkTime {
        CkTime::new(-self.ms)
    }
}

/// Configuration for formatting a time string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CkTimeFormat {
    /// Decimal separator character (`.` or `:`).
    pub decimal_point: char,
    /// Number of decimal places: 3 = ms, 2 = centiseconds. `0` = no fraction.
    pub decimal_places: u8,
    /// Bracket character: `[`, `{`, `(`; `'\0'` = no brackets.
    pub bracket: char,
}

impl CkTimeFormat {
    /// `[00:12.345]` — standard LRC style.
    pub const fn standard() -> Self {
        Self {
            decimal_point: '.',
            decimal_places: 3,
            bracket: '[',
        }
    }

    /// `[00:12:34]` — NicoKaraMaker3 style (centisecond precision).
    pub const fn nicokara() -> Self {
        Self {
            decimal_point: ':',
            decimal_places: 2,
            bracket: '[',
        }
    }
}

impl Default for CkTimeFormat {
    fn default() -> Self {
        Self::standard()
    }
}

/// Render a time to a string, e.g. `[00:12.345]` with [`CkTimeFormat::standard`].
///
/// Fractional digits beyond three are truncated; if `decimal_places` exceeds
/// three, the fraction is zero-padded. A negative time is prefixed with `-`
/// inside the brackets.
pub fn format_time(t: CkTime, fmt: CkTimeFormat) -> String {
    let abs_ms = t.abs_ms();
    let negative = t.is_negative();

    const MS_PER_MIN: i64 = 60_000;
    const MS_PER_SEC: i64 = 1_000;

    let minutes = abs_ms / MS_PER_MIN;
    let seconds = (abs_ms / MS_PER_SEC) % 60;
    let frac = abs_ms % MS_PER_SEC; // 0..999

    let mut core = format!("{minutes:02}:{seconds:02}");
    if fmt.decimal_places > 0 {
        // Pad ms to 3 digits, then truncate to decimal_places (or pad if >3).
        let raw = format!("{frac:03}");
        let mut frac_str: String = raw.chars().take(fmt.decimal_places as usize).collect();
        if fmt.decimal_places > 3 {
            frac_str.push_str(&"0".repeat((fmt.decimal_places - 3) as usize));
        }
        core.push(fmt.decimal_point);
        core.push_str(&frac_str);
    }
    if negative {
        core.insert(0, '-');
    }

    match fmt.bracket {
        '[' => format!("[{core}]"),
        '{' => format!("{{{core}}}"),
        '(' => format!("({core})"),
        _ => core,
    }
}

/// Parse a time string. Mirrors the C++ parser's accepted forms:
/// `[sign](MM:ss.mmm | ss.mmm | pure ms with optional m/ms/msec/millisecond
/// unit)` with optional surrounding brackets.
///
/// Returns a [`CkError`] with one of the [`codes`] on failure (bad format,
/// unmatched brackets, invalid number, or overflow).
pub fn parse_time(input: &str) -> Result<CkTime, CkError> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        // (?i) makes the ms/msec/millisecond unit case-insensitive (Ms/MS ok).
        regex::Regex::new(
            r"(?i)([(\{\[]?)([+-]?)(?:(?:(\d*)[,.:])?(\d*)[,.:](\d*)|(\d+)(?:m(?:s(?:ec)?|illisec(?:ond)?))?)([)\}\]]?)",
        )
        .expect("valid time regex")
    });

    let caps = re.captures(input).ok_or_else(|| {
        CkError::new(codes::INVALID_FORMAT, format!("unrecognized time format: {input:?}"))
    })?;
    let left = caps.get(1).map_or("", |m| m.as_str());
    let right = caps.get(7).map_or("", |m| m.as_str());

    // Bracket matching: both empty, or matching pair.
    let matched = match (left, right) {
        ("", "") => true,
        ("[", "]") | ("{", "}") | ("(", ")") => true,
        _ => false,
    };
    if !matched {
        return Err(CkError::new(
            codes::UNMATCHED_BRACKETS,
            format!("unmatched time brackets in {input:?}"),
        ));
    }

    let sign = if caps.get(2).map_or("", |m| m.as_str()) == "-" {
        -1
    } else {
        1
    };

    // Group 6 = pure milliseconds with unit (no colon/dot structure).
    if let Some(pure) = caps.get(6) {
        let ms: i64 = pure
            .as_str()
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                CkError::new(codes::INVALID_NUMBER, "invalid millisecond value")
                    .with_source(e.to_string())
            })?;
        let ms = ms.checked_mul(sign).ok_or_else(|| {
            CkError::new(codes::OVERFLOW, "millisecond value overflows")
        })?;
        return Ok(CkTime::new(ms));
    }

    // Groups 3/4/5 = minutes/seconds/fraction (fraction separators flexible).
    let minutes: i64 = caps.get(3).map_or("", |m| m.as_str()).parse().unwrap_or(0);
    let seconds: i64 = caps.get(4).map_or("", |m| m.as_str()).parse().map_err(
        |e: std::num::ParseIntError| {
            CkError::new(codes::INVALID_NUMBER, "invalid seconds").with_source(e.to_string())
        },
    )?;
    let frac_str: String = caps.get(5).map_or("", |m| m.as_str()).chars().take(3).collect();
    let frac_str = format!("{frac_str:0<3}");
    let frac_ms: i64 = frac_str.parse().map_err(|e: std::num::ParseIntError| {
        CkError::new(codes::INVALID_NUMBER, "invalid fractional part").with_source(e.to_string())
    })?;

    const MS_PER_SEC: i64 = 1_000;
    const MS_PER_MIN: i64 = 60 * MS_PER_SEC;

    let total = minutes
        .checked_mul(MS_PER_MIN)
        .and_then(|m| m.checked_add(seconds.checked_mul(MS_PER_SEC)?))
        .and_then(|m| m.checked_add(frac_ms))
        .and_then(|m| m.checked_mul(sign))
        .ok_or_else(|| CkError::new(codes::OVERFLOW, "time value overflows"))?;
    Ok(CkTime::new(total))
}

/// Error codes used by time parsing.
pub mod codes {
    use necokara_error::ck_code;

    /// The string did not match any accepted time format.
    pub const INVALID_FORMAT: &str = ck_code!("necokara-lyrics", ck_time, invalid_format);
    /// Brackets were present but did not match (e.g. `[12:34`).
    pub const UNMATCHED_BRACKETS: &str =
        ck_code!("necokara-lyrics", ck_time, unmatched_brackets);
    /// A numeric component could not be parsed.
    pub const INVALID_NUMBER: &str = ck_code!("necokara-lyrics", ck_time, invalid_number);
    /// The parsed value overflowed i64 milliseconds.
    pub const OVERFLOW: &str = ck_code!("necokara-lyrics", ck_time, overflow);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_lrc() {
        // [mm:ss.mmm]
        assert_eq!(parse_time("[00:12.345]").unwrap(), CkTime::new(12_345));
        assert_eq!(parse_time("00:12.345").unwrap(), CkTime::new(12_345));
    }

    #[test]
    fn parse_nicokara_centisecond() {
        // [mm:ss:cc] — centisecond colons
        assert_eq!(parse_time("[01:05:75]").unwrap(), CkTime::new(65_750));
    }

    #[test]
    fn parse_pure_ms_with_unit_case_insensitive() {
        assert_eq!(parse_time("500ms").unwrap(), CkTime::new(500));
        assert_eq!(parse_time("500MS").unwrap(), CkTime::new(500));
        assert_eq!(parse_time("1msec").unwrap(), CkTime::new(1));
        assert_eq!(parse_time("2millisecond").unwrap(), CkTime::new(2));
    }

    #[test]
    fn parse_negative() {
        assert_eq!(parse_time("[-5.000]").unwrap(), CkTime::new(-5_000));
        assert!(parse_time("[-5.000]").unwrap().is_negative());
    }

    #[test]
    fn parse_fraction_truncation() {
        // More than 3 fraction digits truncates to ms.
        assert_eq!(parse_time("[00:01.9999]").unwrap(), CkTime::new(1_999));
    }

    #[test]
    fn parse_invalid_is_err() {
        assert!(parse_time("").is_err());
        assert!(parse_time("abc").is_err());
        assert!(parse_time("[00:12.345").is_err()); // unmatched bracket
        assert!(parse_time("00:12.345]").is_err());
    }

    #[test]
    fn error_codes_distinguish_kinds() {
        use crate::ck_time::codes as c;
        // Unrecognized input -> invalid format.
        assert_eq!(parse_time("abc").unwrap_err().code, c::INVALID_FORMAT);
        // Mismatched bracket -> unmatched brackets.
        assert_eq!(
            parse_time("[00:12.345").unwrap_err().code,
            c::UNMATCHED_BRACKETS
        );
    }

    #[test]
    fn format_roundtrip() {
        let t = CkTime::new(65_750);
        let s = format_time(t, CkTimeFormat::standard());
        assert_eq!(s, "[01:05.750]");
        assert_eq!(parse_time(&s).unwrap(), t);

        let s2 = format_time(t, CkTimeFormat::nicokara());
        assert_eq!(s2, "[01:05:75]");
    }

    #[test]
    fn format_negative_and_no_bracket() {
        let t = CkTime::new(-5_000);
        let fmt = CkTimeFormat { bracket: '\0', ..CkTimeFormat::standard() };
        assert_eq!(format_time(t, fmt), "-00:05.000");
    }

    #[test]
    fn format_zero() {
        assert_eq!(format_time(CkTime::ZERO, CkTimeFormat::standard()), "[00:00.000]");
    }
}
