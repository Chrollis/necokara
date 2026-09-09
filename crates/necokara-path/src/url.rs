//! `crl://` URL syntax for tree-path material references.
//!
//! All materials are referenced by their project material-tree path:
//!   `crl://<tree path>`   e.g. `crl://songs/op/audio`
//!
//! There is no kind/authority segment — every reference names a material (or
//! folder) in the tree. Keys are names / tree segments (no escaping needed).

use necokara_error::CkError;

/// Error codes used by crl url parsing.
pub mod codes {
    use necokara_error::ck_code;

    /// The string is not a valid `crl://<path>` reference.
    pub const INVALID_CRL: &str = ck_code!("necokara-path", url, invalid_crl);
}

/// Parse a `crl://` url, returning the tree path (`songs/op/audio`).
pub fn parse_crl(url: &str) -> Result<String, CkError> {
    let path = url
        .strip_prefix("crl://")
        .ok_or_else(|| CkError::new(codes::INVALID_CRL, format!("not a crl url: {url:?}")))?;
    if path.is_empty() {
        return Err(CkError::new(codes::INVALID_CRL, "empty crl url"));
    }
    Ok(path.to_string())
}

/// Build a `crl://` url from a tree path.
pub fn crl_url(tree_path: &str) -> String {
    format!("crl://{tree_path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_build() {
        assert_eq!(
            parse_crl("crl://songs/op/audio").as_deref(),
            Ok("songs/op/audio")
        );
        assert_eq!(crl_url("songs/op/audio"), "crl://songs/op/audio");
    }

    #[test]
    fn invalid() {
        assert!(parse_crl("crl://").is_err());
        assert!(parse_crl("file:///x").is_err());
    }
}
