//! Necokara unified error type and error-code constants.
//!
//! Every error carries a machine-readable code (`"<pkg>.<module>.<leaf>"`,
//! e.g. `necokara-timing.separate.spawn_failed`) plus a human-readable
//! `what` message. Codes are `&'static str` constants produced by the
//! [`ck_code!`] macro; each crate keeps its own code constants so the full
//! list is discoverable per crate (for docs / frontend lookup).

use std::fmt;
use std::error::Error as StdError;

/// A Necokara error with a machine code and a human message.
///
/// Serialization across the IPC boundary is a translation-layer concern; this
/// type itself is plain.
#[derive(Debug, Clone)]
pub struct CkError {
    /// Machine-readable code, e.g. `"necokara-timing.separate.spawn_failed"`.
    pub code: &'static str,
    /// Human-readable message.
    pub what: String,
    /// Underlying cause when present.
    pub source: Option<String>,
}

impl CkError {
    /// Build an error from a code constant and a message.
    pub fn new(code: &'static str, what: impl Into<String>) -> Self {
        Self {
            code,
            what: what.into(),
            source: None,
        }
    }

    /// Attach an underlying cause message.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl fmt::Display for CkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.what)
    }
}

impl StdError for CkError {}

impl PartialEq for CkError {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}

impl Eq for CkError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_builds_hyphenated_package_code() {
        let code = ck_code!("necokara-timing", separate, spawn_failed);
        assert_eq!(code, "necokara-timing.separate.spawn_failed");
    }

    #[test]
    fn display_and_eq() {
        let a = CkError::new(
            ck_code!("necokara-timing", align, no_tokens),
            "no tokens",
        );
        let b = CkError::new(
            ck_code!("necokara-timing", align, no_tokens),
            "different msg, same code",
        );
        // Equality compares codes, not messages.
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "necokara-timing.align.no_tokens: no tokens");
    }
}

/// Build a `&'static str` error code.
///
/// The first argument is the package name literal (`"necokara-timing"`),
/// followed by module and leaf segments:
///
/// ```
/// # use necokara_error::ck_code;
/// let code = ck_code!("necokara-timing", separate, spawn_failed);
/// assert_eq!(code, "necokara-timing.separate.spawn_failed");
/// ```
#[macro_export]
macro_rules! ck_code {
    ($pkg:literal $(, $seg:ident)+) => {
        concat!(
            $pkg,
            $( ".", stringify!($seg), )+
        )
    };
}
