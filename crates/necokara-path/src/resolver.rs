//! PathResolver: resolves `crl://` tree-path references to real files, and
//! manages the project material tree.
//!
//! The resolver owns the [`MaterialTree`]. A `crl://<tree path>` reference is
//! resolved by walking the tree to the material node and returning its real
//! file path (abs preferred, rel fallback). When the file is missing, offline
//! search reports candidates (fp matches preferred over name matches);
//! relocation itself is left to the caller/user.

use std::path::{Path, PathBuf};

use crate::material_tree::codes as tree_codes;
use crate::material_tree::{MaterialTree, TreeNode};
use crate::search::{find_by_ext_and_fp, find_by_name_many, SearchResult};
use crate::url::parse_crl;
use necokara_error::CkError;

/// Resolver for one project's material tree.
#[derive(Debug, Clone)]
pub struct PathResolver {
    project_root: PathBuf,
    tree: MaterialTree,
}

impl PathResolver {
    /// New resolver rooted at `project_root`, with an empty material tree.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            tree: MaterialTree::new(),
        }
    }

    /// New resolver rooted at `project_root`, adopting a prebuilt tree.
    pub fn with_tree(project_root: impl Into<PathBuf>, tree: MaterialTree) -> Self {
        Self {
            project_root: project_root.into(),
            tree,
        }
    }

    /// The project root.
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// The material tree (read-only).
    pub fn tree(&self) -> &MaterialTree {
        &self.tree
    }

    /// The material tree (mutable).
    pub fn tree_mut(&mut self) -> &mut MaterialTree {
        &mut self.tree
    }

    // ---- crl:// mutating operations ------------------------------------
    //
    // Inputs are full `crl://` references. Folders conventionally end with a
    // trailing `/` (`crl://a/b/`); materials do not. The trailing slash of a
    // folder reference is dropped before tree lookup. Missing parent folders
    // are auto-created; name collisions get a ` (n)` suffix instead of an
    // error.

    /// Strip the `crl://` prefix and return the tree path (dropping a
    /// trailing slash, which merely marks a folder reference).
    fn crl_tree_path(crl: &str) -> Result<String, CkError> {
        let path = parse_crl(crl)?;
        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(CkError::new(
                tree_codes::INVALID_PATH,
                "crl url has no tree path",
            ));
        }
        Ok(trimmed.to_string())
    }

    /// Create a folder at `crl://.../` (trailing slash conventional).
    /// Returns the actual created crl path (may carry a ` (n)` suffix).
    pub fn create_folder(&mut self, crl: &str) -> Result<String, CkError> {
        let path = Self::crl_tree_path(crl)?;
        let created = self.tree.create_folder(&path)?;
        Ok(format!("crl://{created}/"))
    }

    /// Create a material at `crl://...` referencing the file at `abs`
    /// (optional `rel`). The fingerprint is computed here from the file.
    /// Returns the actual created crl path.
    pub fn create_material(
        &mut self,
        crl: &str,
        abs: &str,
        rel: Option<&str>,
    ) -> Result<String, CkError> {
        let path = Self::crl_tree_path(crl)?;
        let fingerprint = crate::fingerprint::fingerprint_file(Path::new(abs));
        let created = self.tree.create_material(&path, abs, rel)?;

        // Fill the computed fingerprint into the created leaf.
        if let Some(TreeNode::Material { fingerprint: fp, .. }) = self.tree.find_by_path_mut(&created)
        {
            *fp = fingerprint;
        }
        Ok(format!("crl://{created}"))
    }

    /// Remove the folder at `crl://.../` (recursively over the tree only).
    pub fn remove_folder(&mut self, crl: &str) -> Result<(), CkError> {
        let path = Self::crl_tree_path(crl)?;
        self.tree.remove_folder(&path)
    }

    /// Remove the material at `crl://...`.
    pub fn remove_material(&mut self, crl: &str) -> Result<(), CkError> {
        let path = Self::crl_tree_path(crl)?;
        self.tree.remove_material(&path)
    }

    /// Rename a node. `new_rel_crl` is a `crl://` path relative to the node's
    /// parent (a plain new name, or one embedding folders that are
    /// auto-created). Returns the new full crl.
    pub fn rename(&mut self, crl: &str, new_rel_crl: &str) -> Result<String, CkError> {
        let path = Self::crl_tree_path(crl)?;
        let new_rel = Self::crl_tree_path(new_rel_crl)?;
        // new_rel is relative to the parent; tree.rename receives only the
        // name part. crl_tree_path dropped the prefix/trailing slash, which is
        // exactly the relative form expected.
        let renamed = self.tree.rename(&path, &new_rel)?;
        Ok(format!("crl://{renamed}"))
    }

    /// Copy a node (folder recurses) to the target crl.
    pub fn copy(&mut self, src_crl: &str, dst_crl: &str) -> Result<String, CkError> {
        let src = Self::crl_tree_path(src_crl)?;
        let dst = Self::crl_tree_path(dst_crl)?;
        let copied = self.tree.copy(&src, &dst)?;
        Ok(format!("crl://{copied}"))
    }

    /// Move a node to the target crl.
    pub fn move_(&mut self, src_crl: &str, dst_crl: &str) -> Result<String, CkError> {
        let src = Self::crl_tree_path(src_crl)?;
        let dst = Self::crl_tree_path(dst_crl)?;
        let moved = self.tree.move_(&src, &dst)?;
        Ok(format!("crl://{moved}"))
    }

    /// Resolve a `crl://<tree path>` reference to the material's real file
    /// path.
    ///
    /// Errors when the url is malformed, the tree path is missing, the node is
    /// a folder, or the referenced file does not exist (abs/rel both fail).
    pub fn resolve(&self, url: &str) -> Result<PathBuf, CkError> {
        let tree_path = parse_crl(url)?;
        self.resolve_tree_path(&tree_path)
    }

    /// Resolve a tree path to an existing file (abs preferred, rel fallback).
    pub fn resolve_tree_path(&self, tree_path: &str) -> Result<PathBuf, CkError> {
        let node = self
            .tree
            .find_by_path(tree_path)
            .ok_or_else(|| CkError::new(tree_codes::NOT_FOUND, format!("{tree_path} not in tree")))?;
        let TreeNode::Material {
            path_abs,
            path_rel,
            ..
        } = node
        else {
            return Err(CkError::new(
                tree_codes::TYPE_MISMATCH,
                format!("{tree_path} is a folder, not a material"),
            ));
        };

        let abs = Path::new(path_abs);
        if abs.is_file() {
            return Ok(abs.to_path_buf());
        }
        if let Some(rel) = path_rel {
            let p = self.project_root.join(rel);
            if p.is_file() {
                return Ok(p);
            }
        }
        Err(CkError::new(
            tree_codes::NOT_FOUND,
            format!("file for {tree_path} not found on disk"),
        ))
    }

    /// Run an offline search for the material at `tree_path` and return all
    /// candidates as a [`SearchResult`] (`by_fp` preferred, `by_name` as a
    /// fallback — the caller decides which to use).
    ///
    /// Callers should first try [`PathResolver::resolve_tree_path`]; this
    /// method only performs the search for a missing file.
    pub fn resolve_with_offline(&self, tree_path: &str) -> SearchResult {
        let empty = SearchResult::default();
        let node = match self.tree.find_by_path(tree_path) {
            Some(n) => n,
            None => return empty,
        };
        let TreeNode::Material {
            path_abs,
            fingerprint,
            ..
        } = node
        else {
            return empty;
        };

        // Candidate roots: original parent directory + project root.
        let original = Path::new(path_abs);
        let mut roots = Vec::new();
        if let Some(parent) = original.parent() {
            if parent.is_dir() {
                roots.push(parent.to_path_buf());
            }
        }
        roots.push(self.project_root.clone());

        let file_name = original
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let ext = original
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let fp_matches = fingerprint
            .as_deref()
            .map(|fp| find_by_ext_and_fp(&roots, ext, fp))
            .unwrap_or_default();
        let name_matches = find_by_name_many(file_name, &roots);

        SearchResult {
            by_fp: fp_matches,
            by_name: name_matches,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a resolver whose tree has one material at "audio/track"
    /// pointing at `abs_path`.
    fn resolver_with_material(project_root: &Path, abs_path: &Path) -> PathResolver {
        let mut resolver = PathResolver::new(project_root);
        resolver.create_folder("crl://audio/").unwrap();
        resolver
            .create_material(
                "crl://audio/track",
                &abs_path.to_string_lossy(),
                None,
            )
            .unwrap();
        resolver
    }

    #[test]
    fn resolve_existing_abs_file() {
        let dir = std::env::temp_dir().join("nk_path_test_resolve");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("song.mp3");
        std::fs::write(&file, b"abc").unwrap();

        let resolver = resolver_with_material(&dir, &file);
        let got = resolver.resolve("crl://audio/track").unwrap();
        assert_eq!(got, file);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_missing_file_is_err() {
        let dir = std::env::temp_dir().join("nk_path_test_missing");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();

        let missing = dir.join("nope.mp3");
        let resolver = resolver_with_material(&dir, &missing);
        assert!(resolver.resolve("crl://audio/track").is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_rel_fallback_when_abs_gone() {
        let root = std::env::temp_dir().join("nk_path_test_rel_root");
        std::fs::remove_dir_all(&root).ok();
        let sub = root.join("media");
        std::fs::create_dir_all(&sub).unwrap();
        let rel_file = sub.join("song.mp3");
        std::fs::write(&rel_file, b"data").unwrap();

        let mut resolver = PathResolver::new(&root);
        resolver
            .create_material(
                "crl://track",
                &root.join("gone/song.mp3").to_string_lossy(),
                Some("media/song.mp3"),
            )
            .unwrap();

        let got = resolver.resolve("crl://track").unwrap();
        assert_eq!(got, rel_file);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn offline_search_finds_moved_file_in_original_parent() {
        let dir = std::env::temp_dir().join("nk_path_test_offline");
        // Clean any leftovers from a previous (possibly failed) run.
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();

        // The material points at a file that no longer exists...
        let gone = dir.join("song.mp3");
        let resolver = resolver_with_material(&dir, &gone);
        assert!(resolver.resolve_tree_path("audio/track").is_err());

        // ...but a same-named file now sits in the original parent dir.
        let moved = dir.join("song.mp3");
        std::fs::write(&moved, b"x").unwrap();

        let result = resolver.resolve_with_offline("audio/track");
        // Name channel finds it (no fingerprint recorded on the entry).
        assert!(result.by_fp.is_empty());
        assert_eq!(result.by_name, vec![moved]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn offline_search_no_candidates_is_empty() {
        let dir = std::env::temp_dir().join("nk_path_test_offline_none");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();

        let gone = dir.join("nowhere.mp3");
        let resolver = resolver_with_material(&dir, &gone);

        let result = resolver.resolve_with_offline("audio/track");
        assert!(result.by_fp.is_empty());
        assert!(result.by_name.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
