//! Python wrapper for BPM detection (librosa), vocal separation (demucs)
//! and lyric alignment (stable-ts). Spawns the bundled python interpreter
//! running the scripts in `python/scripts/` through `necokara-spawn`, then
//! maps the returned JSON payload onto the local result types.
//!
//! Transport failures (spawn / exit status / envelope) are reported by
//! `necokara-spawn.*` codes; the payload shapes below only need to fail when
//! the data itself is malformed.

use std::path::Path;

use necokara_error::CkError;
use necokara_lyrics::ck_time::CkTime;
use necokara_spawn::ProcessInput;

use crate::bpm_list::BpmList;

/// Result of one aligned character.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignChar {
    /// The character.
    pub ch: char,
    /// Start time in seconds (as reported by stable-ts).
    pub time: f64,
}

/// Outputs of a successful separation.
#[derive(Debug, Clone)]
pub struct SeparateOutput {
    /// Absolute path of the vocals wav.
    pub vocals_path: std::path::PathBuf,
    /// Absolute path of the accompaniment wav.
    pub accompaniment_path: std::path::PathBuf,
}

/// Separate `src` into vocals + accompaniment wavs.
///
/// Runs `separate.py --src <src> --vocals <vocals> --accompaniment <acc>
/// --model-dir <model_dir> --ffmpeg <ffmpeg>`.
pub fn separate(
    python: &Path,
    script: &Path,
    ffmpeg: &Path,
    model_dir: &Path,
    src: &Path,
    vocals_out: &Path,
    accompaniment_out: &Path,
) -> Result<SeparateOutput, CkError> {
    necokara_spawn::run_python_json(
        python,
        script,
        &[
            "--src",
            src.to_str().unwrap_or_default(),
            "--vocals",
            vocals_out.to_str().unwrap_or_default(),
            "--accompaniment",
            accompaniment_out.to_str().unwrap_or_default(),
            "--model-dir",
            model_dir.to_str().unwrap_or_default(),
            "--ffmpeg",
            ffmpeg.to_str().unwrap_or_default(),
        ],
        ProcessInput::None,
    )?;
    Ok(SeparateOutput {
        vocals_path: vocals_out.to_path_buf(),
        accompaniment_path: accompaniment_out.to_path_buf(),
    })
}

/// Force-align `lyrics` onto `vocals_wav`; returns per-character times.
///
/// Runs `align.py --vocals <wav> --lyrics <text> --lang <lang>
/// --model-dir <dir> --ffmpeg <ffmpeg>` and maps the `tokens` array onto
/// [`AlignChar`]. The caller decides how to apply the times; this function
/// only returns them.
pub fn align(
    python: &Path,
    script: &Path,
    ffmpeg: &Path,
    model_dir: &Path,
    vocals_wav: &Path,
    lyrics: &str,
    lang: &str,
) -> Result<Vec<AlignChar>, CkError> {
    let payload = necokara_spawn::run_python_json(
        python,
        script,
        &[
            "--vocals",
            vocals_wav.to_str().unwrap_or_default(),
            "--lyrics",
            lyrics,
            "--lang",
            lang,
            "--model-dir",
            model_dir.to_str().unwrap_or_default(),
            "--ffmpeg",
            ffmpeg.to_str().unwrap_or_default(),
        ],
        ProcessInput::None,
    )?;

    let mut chars = Vec::new();
    if let Some(tokens) = payload.get("tokens").and_then(|v| v.as_array()) {
        for tok in tokens {
            let time = tok.get("time").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let ch = tok
                .get("char")
                .and_then(|v| v.as_str())
                .and_then(|s| s.chars().next())
                .unwrap_or('\u{FFFD}');
            chars.push(AlignChar { ch, time });
        }
    }
    Ok(chars)
}

/// Detect BPM segments over an audio file via the librosa script
/// (`bpm.py`), returning the parsed [`BpmList`].
///
/// Runs
/// `bpm.py --input <audio> --ffmpeg <ffmpeg> --threshold <t> --window <w>
/// --merge-ratio <r>` and maps its `segments` (`{bpm, start_ms}`) onto a
/// fresh [`BpmList`]. Segment starts are real beat positions, so a beat grid
/// can be drawn from `start + k * 60000/bpm`.
pub fn bpm_detect(
    python: &Path,
    script: &Path,
    ffmpeg: &Path,
    audio: &Path,
    threshold: f64,
    window: usize,
    merge_ratio: f64,
) -> Result<BpmList, CkError> {
    let payload = necokara_spawn::run_python_json(
        python,
        script,
        &[
            "--input",
            audio.to_str().unwrap_or_default(),
            "--ffmpeg",
            ffmpeg.to_str().unwrap_or_default(),
            "--threshold",
            &threshold.to_string(),
            "--window",
            &window.to_string(),
            "--merge-ratio",
            &merge_ratio.to_string(),
        ],
        ProcessInput::None,
    )?;

    let mut list = BpmList::new();
    if let Some(segs) = payload.get("segments").and_then(|v| v.as_array()) {
        for seg in segs {
            let start_ms = seg
                .get("start_ms")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let bpm = seg.get("bpm").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if bpm > 0.0 {
                list.insert(CkTime::new(start_ms), bpm);
            }
        }
    }
    Ok(list)
}

/// List the language codes supported by the whisper model in `model_dir`
/// via the `ai-languages.py` script.
///
/// Runs `ai-languages.py --model-dir <dir>` and returns the `languages`
/// array of codes (e.g. `["en", "zh", "ja", ...]`). Only the tokenizer
/// vocabulary is read, so this is fast.
pub fn ai_languages(
    python: &Path,
    script: &Path,
    model_dir: &Path,
) -> Result<Vec<String>, CkError> {
    let payload = necokara_spawn::run_python_json(
        python,
        script,
        &["--model-dir", model_dir.to_str().unwrap_or_default()],
        ProcessInput::None,
    )?;

    let mut codes = Vec::new();
    if let Some(list) = payload.get("languages").and_then(|v| v.as_array()) {
        for item in list {
            if let Some(code) = item.as_str() {
                codes.push(code.to_string());
            }
        }
    }
    Ok(codes)
}
