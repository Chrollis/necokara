# necokara-error

Unified error type and error codes for Necokara crates.

## What it does

- `CkError { code, what, source }` — one error type shared by every crate.
- `ck_code!("necokara-timing", bpm_list, not_found)` builds the code string
  `"necokara-timing.bpm_list.not_found"`.
- Each module exposes its codes as `pub mod codes` constants, so the full
  list is discoverable per crate (for docs / frontend lookup).

Errors serialize across the IPC boundary at the translation layer; this type
is plain.

## Example

```rust
use necokara_error::{ck_code, CkError};

let e = CkError::new(ck_code!("necokara-timing", align, no_tokens), "no tokens");
assert_eq!(e.code, "necokara-timing.align.no_tokens");
```
