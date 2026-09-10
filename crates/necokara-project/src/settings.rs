//! Project settings: values that participate in computation or behaviour.
//!
//! Descriptive-only information (song/project metadata) lives in
//! [`Metadata`] (crate::metadata). This structure holds everything that
//! affects logic: material/page dimensions, export canvas & encoding, and
//! per-project session flags.

use necokara_lyrics::ck_time::CkTime;

/// Material / subtitle-track dimensions: the lyric design area (the "page").
/// This is the source material size; the export canvas may differ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSettings {
    /// Page (material) width in pixels.
    pub width_px: u32,
    /// Page (material) height in pixels.
    pub height_px: u32,
    /// Frame rate used by the editor's simple preview.
    pub frame_rate: f64,
}

impl Default for PageSettings {
    fn default() -> Self {
        Self {
            width_px: 1920,
            height_px: 1080,
            frame_rate: 30.0,
        }
    }
}

/// Export/output canvas dimensions: the final video size. The source material
/// is scaled (up to same-ratio) to fit this.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasSettings {
    /// Output width in pixels.
    pub width_px: u32,
    /// Output height in pixels.
    pub height_px: u32,
    /// Output frame rate (used for real preview rendering and export).
    pub frame_rate: f64,
}

impl Default for CanvasSettings {
    fn default() -> Self {
        Self {
            width_px: 1920,
            height_px: 1080,
            frame_rate: 30.0,
        }
    }
}

/// Encoding settings for export.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    /// Container format.
    pub container: String,
    /// Video codec name.
    pub video_codec: String,
    /// Rate-control mode: `crf` or `bitrate`.
    pub video_mode: String,
    /// CRF value (0-51) when `video_mode = crf`.
    pub crf: u32,
    /// Video bitrate string when `video_mode = bitrate` (e.g. "8M").
    pub video_bitrate: String,
    /// Audio codec name.
    pub audio_codec: String,
    /// Audio bitrate string.
    pub audio_bitrate: String,
    /// Export range in-point (ms).
    pub in_point: CkTime,
    /// Export range out-point (ms).
    pub out_point: CkTime,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            container: "mp4".to_string(),
            video_codec: "libx264".to_string(),
            video_mode: "crf".to_string(),
            crf: 20,
            video_bitrate: String::new(),
            audio_codec: "aac".to_string(),
            audio_bitrate: "192k".to_string(),
            in_point: CkTime::ZERO,
            out_point: CkTime::new(120_000),
        }
    }
}

/// Per-project session state (follows the project file).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionSettings {
    /// Optional user password (plaintext for now; encryption TBD).
    pub user_password: Option<String>,
    /// "Don't ask again" flags migrated from the Electron reference.
    pub skip_set_password_prompt: bool,
    pub skip_change_password_prompt: bool,
    pub skip_effect_import_prompt: bool,
    pub skip_effect_import_reset: bool,
}

/// Project settings: everything that affects computation or behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Material / subtitle-track (page) dimensions.
    pub page: PageSettings,
    /// Export canvas dimensions.
    pub canvas: CanvasSettings,
    /// Export encoding + range.
    pub export: ExportSettings,
    /// Per-project session state.
    pub session: SessionSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            page: PageSettings::default(),
            canvas: CanvasSettings::default(),
            export: ExportSettings::default(),
            session: SessionSettings::default(),
        }
    }
}
