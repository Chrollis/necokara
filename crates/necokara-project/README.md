# necokara-project

Project-level structures for Necokara.

## What it does

Holds the two project-level data groups:

- **metadata** (`metadata.rs`) - purely descriptive information:
  - `LyricsInfo` - song info (`@Title`/`@Artist`/`@Lyrics`(au)/`@Compose`/...)
  - `ProjectInfo` - file/project info (created/updated, renderer, version)
  - `Metadata { lyrics, project, extra }`
- **settings** (`settings.rs`) - everything that affects computation or
  behaviour:
  - `PageSettings` (material/subtitle-track dimensions)
  - `CanvasSettings` (export canvas)
  - `ExportSettings` (container/codec/crf/range)
  - `SessionSettings` (password + "don't ask again" flags)
  - `Settings` grouping them

The metadata/settings split follows the rule: descriptive info goes to
metadata, anything that participates in logic goes to settings.

Depends on `necokara-lyrics` (base types) and `necokara-timing`. Serialization
is a separate translation layer.

## Usage sketch

```rust
use necokara_project::{Metadata, Settings};

let metadata = Metadata::default();
let settings = Settings::default();
```
