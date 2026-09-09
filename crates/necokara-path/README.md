# necokara-path

Media path resolution and material management for Necokara.

## What it does

`necokara-path` manages a project's **material tree** - the virtual,
user-organizable tree of local media files - and maps `crl://` references to
real files on disk.

Everything is a local file. A `crl://` reference is just a tree path:

```
crl://songs/op/audio        ->  material tree path  songs/op/audio
```

## Scope

This crate only resolves *what* a reference points to. It deliberately does
**not**:

- read/decode media (`MediaSource` implementations, frame/audio access -
  those belong to the composition/rendering layer);
- manage virtual materials (nested sequences, subtitles, solids);
- cache derived products;
- auto-relocate files.

It is independent of `necokara-lyrics` and of Tauri.

## Modules

| Module | Purpose |
|---|---|
| `material_tree` | The virtual material tree: `MaterialTree` (single root folder) with `TreeNode` (folder / material), unique sibling names, insert/remove by tree path |
| `url` | `crl://` URL syntax: parse a reference to a tree path, build a URL from a path |
| `resolver` | `PathResolver`: holds the tree, resolves tree paths to real files (abs first, rel fallback), validates, runs offline search |
| `search` | Offline search: find missing files by fingerprint (preferred) or by file name over candidate directories |
| `fingerprint` | Cheap content fingerprint (head/tail bytes + size -> sha256 hex) for validation |

## Usage sketch

```rust
use necokara_path::{MaterialTree, PathResolver, TreeNode, crl_url};

// Build a resolver for a project.
let resolver = PathResolver::new("C:/projects/my-song");

// Insert a material into the tree at a virtual path.
let node = TreeNode::Material {
    name: "audio".to_string(),
    path_abs: "C:/media/op_audio.mp3".to_string(),
    path_rel: Some("materials/op_audio.mp3".to_string()),
    fingerprint: None,
};
resolver.insert_node(node, "songs/op/audio");

// Resolve a crl:// reference to a real file path.
let path = resolver.resolve("crl://songs/op/audio");
```

## Offline search

When a material's file is moved away, `resolve_with_offline` searches
candidate directories (the file's original parent and the project root):

- files with the same extension whose **fingerprint** matches (the reliable
  "same file" match) are preferred;
- otherwise files with the same **name** are reported.

Results come back as a `SearchResult { by_fp, by_name }`; relocation itself is
left to the caller (the frontend asks the user to confirm).

## Validation

`fingerprint_file(path)` produces a cheap sha256 fingerprint from the first
and last 64 KiB plus the file size, so a referenced file can be checked
against the fingerprint recorded on import.
