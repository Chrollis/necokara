//! Necokara project-level structures.
//!
//! Project metadata (descriptive song/project info) and settings (everything
//! that affects computation/behaviour) live here. Depends on necokara-lyrics for
//! base types and on necokara-timing for timing structures used by settings.

pub mod metadata;
pub mod settings;

pub use metadata::{LyricsInfo, Metadata, ProjectInfo};
pub use settings::{
    CanvasSettings, ExportSettings, PageSettings, SessionSettings, Settings,
};
