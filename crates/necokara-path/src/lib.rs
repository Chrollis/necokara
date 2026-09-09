//! Necokara media path resolution and material management.
//!
//! `necokara-path` manages a project's *material tree* (the virtual,
//! user-organizable tree of local media files) and resolves `crl://` tree
//! path references to real files. It also validates materials (fingerprint)
//! and provides offline search candidates when a referenced file is missing.
//!
//! Out of scope by design: virtual materials (nested sequences, subtitles,
//! solids) and media reading/rendering (`MediaSource` implementations) belong
//! to the composition/rendering layer. This crate only resolves *what* a
//! reference points to, not how its content is produced or consumed.
//!
//! Independent of necokara-lyrics and of Tauri.

pub mod fingerprint;
pub mod material_tree;
pub mod resolver;
pub mod search;
pub mod url;

pub use fingerprint::fingerprint_file;
pub use material_tree::codes as material_tree_codes;
pub use material_tree::{MaterialTree, TreeNode};
pub use resolver::PathResolver;
pub use search::{candidate_roots, find_by_ext_and_fp, find_by_name_many, SearchResult};
pub use url::codes as url_codes;
pub use url::{crl_url, parse_crl};
