# necokara-spawn

Child-process runner for Necokara.

## What it does

Necokara ships helper binaries — a bundled Python interpreter now, ffmpeg
later — that are invoked as one-shot child processes. `necokara-spawn` owns the
shared transport instead of every crate re-implementing it:

- build and spawn the child with captured stdout / stderr;
- optionally feed it argv, raw bytes, or a JSON document on stdin, then close
  stdin so the child sees EOF;
- check the exit status;
- unwrap the `{"ok": true, ...}` / `{"ok": false, "error": ...}` envelope that
  helper scripts print on stdout;
- report every transport failure with a `necokara-spawn.*` code, so the
  transport layer is distinguishable from the calling module's own errors.

Helper scripts must keep stdout JSON-only: diagnostics go to stderr.

## Usage

```rust
use std::path::Path;
use necokara_spawn::{run_python_json, ProcessInput};

// argv-only scripts (align / separate / bpm / ai-languages).
let payload = run_python_json(
    Path::new("binaries/python.exe"),
    Path::new("python/scripts/align.py"),
    &["--vocals", "vocals.wav", "--lang", "ja"],
    ProcessInput::None,
)?;

// scripts that take their whole request on stdin (animation presets).
let request = serde_json::json!({ "fps": 30, "lines": [] });
let payload = run_python_json(
    Path::new("binaries/python.exe"),
    Path::new("python/scripts/presets/FadeIn.py"),
    &[],
    ProcessInput::Json(&request),
)?;
```

## API

| Item | Purpose |
|---|---|
| `run(exe, args, input)` | run any executable, return captured `Output` |
| `run_python(python, script, args, input)` | run `python script args...`, return captured `Output` |
| `run_json(exe, args, input)` | run and unwrap the stdout envelope |
| `run_python_json(python, script, args, input)` | run a script and unwrap the stdout envelope |
| `parse_envelope(stdout)` | validate an envelope string without spawning |
| `ProcessInput::{None, Bytes, Json}` | what to feed the child on stdin |

## Errors

All errors are reported with `necokara-spawn.*` codes:

| Code | Meaning |
|---|---|
| `necokara-spawn.process.spawn_failed` | the executable could not be spawned |
| `necokara-spawn.process.process_failed` | the child exited with a non-zero status |
| `necokara-spawn.process.stdin_failed` | serializing or writing stdin failed |
| `necokara-spawn.process.invalid_json` | the child's stdout was not valid JSON |
| `necokara-spawn.process.task_failed` | the child reported `{"ok": false, ...}` |

The child's stderr is attached as the error `source`, so failures stay
diagnosable.

## Notes

- Payloads are written to stdin before waiting, so a child that writes a lot to
  stdout while stdin is still being written could in principle deadlock on a
  full pipe. Requests are small by design; if a caller ever feeds large data,
  move the write to its own thread.
- Output is decoded as UTF-8 with lossy replacement. Scripts are expected to
  write UTF-8 (see the stdin/stdout protocol in
  `develop/DESIGN_v0.4_animation.md`).
- Spawning requires a real machine: the sandbox used during development denies
  piped stdio, so only the pure parsing paths are covered by tests there.

Depends on `necokara-error` (unified errors) and `serde_json`.
