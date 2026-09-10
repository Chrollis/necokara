//! Child-process runner.
//!
//! Necokara ships helper binaries (a bundled Python interpreter now, ffmpeg
//! later) that are invoked as one-shot child processes. This module owns the
//! shared transport:
//!
//! - build and spawn the child with captured stdout / stderr;
//! - optionally feed it argv, raw bytes, or a JSON document on stdin, then
//!   close stdin so the child sees EOF;
//! - check the exit status;
//! - unwrap the `{"ok": true, ...}` / `{"ok": false, "error": ...}` envelope
//!   that our helper scripts print on stdout.
//!
//! All errors use the [`crate::codes`] `necokara-spawn.*` codes, so the
//! transport layer is reported independently of the calling module.
//!
//! Helper scripts must keep stdout JSON-only: diagnostics go to stderr.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use necokara_error::CkError;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// The executable could not be spawned (missing binary, bad path, ...).
    pub const SPAWN_FAILED: &str = ck_code!("necokara-spawn", process, spawn_failed);
    /// The child exited with a non-zero status.
    pub const PROCESS_FAILED: &str = ck_code!("necokara-spawn", process, process_failed);
    /// Writing to the child's stdin failed.
    pub const STDIN_FAILED: &str = ck_code!("necokara-spawn", process, stdin_failed);
    /// The child's stdout was not valid JSON.
    pub const INVALID_JSON: &str = ck_code!("necokara-spawn", process, invalid_json);
    /// The child reported `{"ok": false, ...}`.
    pub const TASK_FAILED: &str = ck_code!("necokara-spawn", process, task_failed);
}

/// What to feed the child on stdin.
#[derive(Debug, Clone, Copy)]
pub enum ProcessInput<'a> {
    /// The child gets no stdin (`Stdio::null`).
    None,
    /// Raw bytes are written to stdin, then the pipe is closed.
    Bytes(&'a [u8]),
    /// The value is serialized as UTF-8 JSON, written to stdin, then the pipe
    /// is closed.
    Json(&'a serde_json::Value),
}

impl ProcessInput<'_> {
    /// The bytes to write to stdin, if any.
    fn bytes(self) -> Result<Option<Vec<u8>>, CkError> {
        match self {
            ProcessInput::None => Ok(None),
            ProcessInput::Bytes(bytes) => Ok(Some(bytes.to_vec())),
            ProcessInput::Json(value) => serde_json::to_vec(value).map(Some).map_err(|e| {
                CkError::new(codes::STDIN_FAILED, "could not serialize stdin JSON")
                    .with_source(e.to_string())
            }),
        }
    }

    /// Whether the child needs a piped stdin.
    fn needs_stdin(self) -> bool {
        !matches!(self, ProcessInput::None)
    }
}

/// Run `exe args...` to completion, capturing stdout and stderr.
pub fn run(exe: &Path, args: &[&str], input: ProcessInput<'_>) -> Result<Output, CkError> {
    spawn_and_collect(exe, None, args, input)
}

/// Run `python script args...` to completion, capturing stdout and stderr.
pub fn run_python(
    python: &Path,
    script: &Path,
    args: &[&str],
    input: ProcessInput<'_>,
) -> Result<Output, CkError> {
    spawn_and_collect(python, Some(script), args, input)
}

/// Run `exe args...` and parse its `{"ok": ...}` stdout envelope.
pub fn run_json(
    exe: &Path,
    args: &[&str],
    input: ProcessInput<'_>,
) -> Result<serde_json::Value, CkError> {
    let output = run(exe, args, input)?;
    envelope_output(exe, output)
}

/// Run `python script args...` and parse its `{"ok": ...}` stdout envelope.
pub fn run_python_json(
    python: &Path,
    script: &Path,
    args: &[&str],
    input: ProcessInput<'_>,
) -> Result<serde_json::Value, CkError> {
    let output = run_python(python, script, args, input)?;
    envelope_output(script, output)
}

/// Parse a helper's stdout envelope, returning the payload on success.
pub fn parse_envelope(stdout: &str) -> Result<serde_json::Value, CkError> {
    let payload: serde_json::Value = serde_json::from_str(stdout).map_err(|e| {
        CkError::new(codes::INVALID_JSON, "child stdout was not JSON").with_source(e.to_string())
    })?;
    if payload.get("ok").and_then(|value| value.as_bool()) != Some(true) {
        let error = payload
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        return Err(CkError::new(codes::TASK_FAILED, "child task reported failure")
            .with_source(error));
    }
    Ok(payload)
}

/// Spawn the child, feed stdin when requested, and collect its output.
fn spawn_and_collect(
    exe: &Path,
    script: Option<&Path>,
    args: &[&str],
    input: ProcessInput<'_>,
) -> Result<Output, CkError> {
    let mut command = Command::new(exe);
    if let Some(script) = script {
        command.arg(script);
    }
    command.args(args);
    command.stdin(if input.needs_stdin() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| {
        CkError::new(codes::SPAWN_FAILED, format!("could not spawn {}", exe.display()))
            .with_source(e.to_string())
    })?;

    if let Some(bytes) = input.bytes()? {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            CkError::new(codes::STDIN_FAILED, "child stdin was not piped")
        })?;
        stdin.write_all(&bytes).map_err(|e| {
            CkError::new(codes::STDIN_FAILED, "could not write to child stdin")
                .with_source(e.to_string())
        })?;
        // Dropping stdin closes the pipe, so the child observes EOF.
    }

    child.wait_with_output().map_err(|e| {
        CkError::new(
            codes::PROCESS_FAILED,
            format!("could not wait for {}", exe.display()),
        )
        .with_source(e.to_string())
    })
}

/// Turn a finished child's output into an envelope result.
fn envelope_output(subject: &Path, output: Output) -> Result<serde_json::Value, CkError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CkError::new(
            codes::PROCESS_FAILED,
            format!("{} exited with {}", subject.display(), output.status),
        )
        .with_source(stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_envelope(&stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_envelope_accepts_ok_true() {
        let payload = parse_envelope(r#"{"ok": true, "tokens": [1, 2]}"#).expect("valid envelope");
        assert_eq!(payload.get("tokens").unwrap().as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_envelope_rejects_ok_false() {
        let err = parse_envelope(r#"{"ok": false, "error": "boom"}"#).unwrap_err();
        assert_eq!(err.code, codes::TASK_FAILED);
        assert_eq!(err.source.as_deref(), Some("boom"));
    }

    #[test]
    fn parse_envelope_rejects_missing_ok() {
        let err = parse_envelope(r#"{"tokens": []}"#).unwrap_err();
        assert_eq!(err.code, codes::TASK_FAILED);
    }

    #[test]
    fn parse_envelope_rejects_non_json() {
        let err = parse_envelope("not json at all").unwrap_err();
        assert_eq!(err.code, codes::INVALID_JSON);
    }

    #[test]
    fn input_none_needs_no_stdin() {
        assert!(!ProcessInput::None.needs_stdin());
        assert!(ProcessInput::Bytes(b"x").needs_stdin());
    }

    #[test]
    fn input_json_serializes_utf8() {
        let value = serde_json::json!({ "text": "歌词" });
        let bytes = ProcessInput::Json(&value).bytes().unwrap().unwrap();
        let text = String::from_utf8(bytes).expect("stdin must be utf-8");
        assert!(text.contains("歌词"));
    }

    #[test]
    fn input_bytes_are_passed_through() {
        let bytes = ProcessInput::Bytes(b"abc").bytes().unwrap().unwrap();
        assert_eq!(bytes, b"abc");
    }

    #[test]
    fn missing_executable_reports_spawn_failed() {
        let err = run(
            Path::new("necokara-definitely-missing-executable-xyz"),
            &[],
            ProcessInput::None,
        )
        .unwrap_err();
        assert_eq!(err.code, codes::SPAWN_FAILED);
    }
}
