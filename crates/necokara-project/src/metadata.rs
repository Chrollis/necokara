//! Project metadata: purely descriptive information (song info, project/file
//! info) plus a free-form `extra` bucket.
//!
//! Anything that participates in computation/logic lives in [`Settings`]
//! (crate::settings), not here. Field names follow the RL/NKM3 time-tag
//! standard (see <https://zenn.dev/shinta0806/articles/time-tag-standard>):
//!   lyrics.title       ↔ `@Title` (LRC `ti`)
//!   lyrics.artist      ↔ `@Artist` (LRC `ar`)
//!   lyrics.album       ↔ `@Album` (LRC `al`)
//!   lyrics.lyricist    ↔ `@Lyrics` (LRC `au` — lyrics author)
//!   lyrics.compose     ↔ `@Compose`
//!   lyrics.arrange     ↔ `@Arrange`
//!   lyrics.year        ↔ `@Year`
//!   lyrics.tagging_by  ↔ `@TaggingBy` (LRC `by` — file creator)
//!   lyrics.edited_by   ↔ `@EditedBy`
//!   project.created_at / updated_at: ISO8601 strings (passed through)
//!   project.renderer / version: generating tool (LRC `re`) / version (LRC `ve`)

use std::collections::BTreeMap;

/// Song information (descriptive only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LyricsInfo {
    /// `@Title` — song title.
    pub title: String,
    /// `@Artist` — artist.
    pub artist: String,
    /// `@Album` — album name.
    pub album: String,
    /// `@Lyrics` / LRC `au` — lyrics author.
    pub lyricist: String,
    /// `@Compose` — composer.
    pub compose: String,
    /// `@Arrange` — arranger.
    pub arrange: String,
    /// `@Year` — release year.
    pub year: String,
    /// `@TaggingBy` — who placed the time tags (LRC `by`).
    pub tagging_by: String,
    /// `@EditedBy` — who edited the tags.
    pub edited_by: String,
}

/// Project / file information (descriptive only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectInfo {
    /// ISO8601 creation time (passed through, not parsed).
    pub created_at: String,
    /// ISO8601 last-update time.
    pub updated_at: String,
    /// Generating tool (LRC `re`).
    pub renderer: String,
    /// Version (LRC `ve`).
    pub version: String,
}

/// Project metadata: grouped descriptive fields plus free-form extras.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// Song information.
    pub lyrics: LyricsInfo,
    /// Project/file information.
    pub project: ProjectInfo,
    /// Unknown / free-form key/value pairs; keys lowercased.
    pub extra: BTreeMap<String, String>,
}
