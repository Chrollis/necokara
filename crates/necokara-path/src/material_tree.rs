//! Project material tree: the virtual, user-organizable tree of local media
//! files.
//!
//! The tree is the project's material library view (shown in the frontend).
//! Each material is a file on disk referenced by a virtual tree path; the
//! tree maps `crl://<tree path>` references to real files.
//!
//! Path conventions (all operations take a `/`-separated path without the
//! `crl://` prefix; the resolver layer strips it):
//!  - folders are addressed with a trailing `/` by the resolver, but tree
//!    paths here ignore a trailing slash for lookup;
//!  - `.` / `..` / empty segments are rejected (invalid path);
//!  - missing parent folders are created automatically by
//!    `create_folder`/`create_material`/`rename` (when the new name embeds a
//!    sub-path) and `copy`/`move_` targets;
//!  - a name collision never fails: names get an automatic ` (n)` suffix
//!    (`song.mp3` -> `song (1).mp3`), tried until unique;
//!  - removal is recursive over the tree only — files on disk are untouched.
//!
//! Errors are [`CkError`]s; per-site error codes live in [`codes`].

use necokara_error::CkError;

/// Error codes used by material-tree operations.
pub mod codes {
    use necokara_error::ck_code;

    /// A parent folder does not exist (operations that do not auto-create
    /// parents only).
    pub const PARENT_NOT_FOUND: &str =
        ck_code!("necokara-path", material_tree, parent_not_found);
    /// The referenced node does not exist.
    pub const NOT_FOUND: &str = ck_code!("necokara-path", material_tree, not_found);
    /// The path contains `.`, `..`, or an empty segment.
    pub const INVALID_PATH: &str = ck_code!("necokara-path", material_tree, invalid_path);
    /// The operation does not match the node type (folder vs material), or a
    /// move would place a folder inside itself.
    pub const TYPE_MISMATCH: &str = ck_code!("necokara-path", material_tree, type_mismatch);
}

/// A node in the material tree: either a folder or a material (file).
#[derive(Debug, Clone, PartialEq)]
pub enum TreeNode {
    /// A folder containing child nodes.
    Folder {
        /// Folder name.
        name: String,
        /// Child nodes.
        children: Vec<TreeNode>,
    },
    /// A material (local file).
    Material {
        /// Material name.
        name: String,
        /// Absolute path to the file (always recorded).
        path_abs: String,
        /// Optional relative path (fallback, relative to the project root).
        path_rel: Option<String>,
        /// Source fingerprint when recorded (used for offline matching).
        fingerprint: Option<String>,
    },
}

impl TreeNode {
    /// The node's own name.
    pub fn name(&self) -> &str {
        match self {
            TreeNode::Folder { name, .. } | TreeNode::Material { name, .. } => name,
        }
    }

    /// Whether this node is a folder.
    pub fn is_folder(&self) -> bool {
        matches!(self, TreeNode::Folder { .. })
    }

    /// Whether this node is a material.
    pub fn is_material(&self) -> bool {
        matches!(self, TreeNode::Material { .. })
    }
}

/// The project material tree: a single rooted folder hierarchy.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaterialTree {
    root: TreeNode,
}

impl Default for TreeNode {
    fn default() -> Self {
        TreeNode::Folder {
            name: String::new(),
            children: Vec::new(),
        }
    }
}

/// Split a `/`-separated path into segments, rejecting `.`/`..`/empty.
fn parse_path(path: &str) -> Result<Vec<String>, CkError> {
    let mut segs = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." | ".." => {
                return Err(CkError::new(
                    codes::INVALID_PATH,
                    format!("invalid path segment {seg:?} in {path:?}"),
                ))
            }
            _ => segs.push(seg.to_string()),
        }
    }
    Ok(segs)
}

impl MaterialTree {
    /// A new empty tree (root folder with an empty name).
    pub fn new() -> Self {
        Self::default()
    }

    /// The root folder.
    pub fn root(&self) -> &TreeNode {
        &self.root
    }

    /// Look up a node by its `/`-separated tree path. Returns `None` when any
    /// segment is missing (an empty path yields the root).
    pub fn find_by_path(&self, path: &str) -> Option<&TreeNode> {
        let mut node = &self.root;
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            node = match node {
                TreeNode::Folder { children, .. } => {
                    children.iter().find(|c| c.name() == seg)?
                }
                _ => return None,
            };
        }
        Some(node)
    }

    /// Look up a mutable node by tree path.
    pub fn find_by_path_mut(&mut self, path: &str) -> Option<&mut TreeNode> {
        let mut node = &mut self.root;
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            node = match node {
                TreeNode::Folder { children, .. } => {
                    children.iter_mut().find(|c| c.name() == seg)?
                }
                _ => return None,
            };
        }
        Some(node)
    }

    /// Whether a path exists (any node type).
    pub fn contains_path(&self, path: &str) -> bool {
        self.find_by_path(path).is_some()
    }

    // ---- Helpers -------------------------------------------------------

    /// Name with a ` (n)` suffix appended before the extension, e.g.
    /// `song.mp3` -> `song (1).mp3`; folders have no extension handling.
    fn suffixed_name(base: &str, n: usize) -> String {
        if let Some((stem, ext)) = base.rsplit_once('.') {
            format!("{stem} ({n}).{ext}")
        } else {
            format!("{base} ({n})")
        }
    }

    /// The first ` (n)`-suffixed name not present among `parent_path`'s
    /// children, starting from `base` itself (checked first).
    fn available_name(&self, parent_path: &str, base: &str) -> Result<String, CkError> {
        let children = match self.find_by_path(parent_path) {
            Some(TreeNode::Folder { children, .. }) => children,
            _ => {
                return Err(CkError::new(
                    codes::PARENT_NOT_FOUND,
                    format!("parent folder {parent_path:?} not found"),
                ))
            }
        };
        let taken = |name: &str| children.iter().any(|c| c.name() == name);
        if !taken(base) {
            return Ok(base.to_string());
        }
        let mut n = 1;
        loop {
            let candidate = Self::suffixed_name(base, n);
            if !taken(&candidate) {
                return Ok(candidate);
            }
            n += 1;
        }
    }

    /// Ensure every folder on `path` exists, creating missing ones.
    /// `path` is the full folder path including its final segment.
    fn ensure_folder_path(&mut self, path: &str) -> Result<(), CkError> {
        let segs = parse_path(path)?;
        let mut node = &mut self.root;
        for seg in &segs {
            let TreeNode::Folder { children, .. } = node else {
                return Err(CkError::new(
                    codes::TYPE_MISMATCH,
                    format!("{seg:?} of {path:?} is not a folder"),
                ));
            };
            if let Some(idx) = children.iter().position(|c| c.name() == seg) {
                node = &mut children[idx];
            } else {
                children.push(TreeNode::Folder {
                    name: seg.clone(),
                    children: Vec::new(),
                });
                node = children.last_mut().unwrap();
            }
        }
        Ok(())
    }

    /// Move or copy a node from `src` onto `dst` (full paths).
    /// `copy: bool` selects copy; a move refuses a folder into its own
    /// subtree.
    fn transfer(
        &mut self,
        src: &str,
        dst: &str,
        copy: bool,
    ) -> Result<String, CkError> {
        let src_segs = parse_path(src)?;
        let dst_segs = parse_path(dst)?;
        if src_segs.is_empty() {
            return Err(CkError::new(codes::NOT_FOUND, "cannot move/copy the root"));
        }
        if !copy {
            // Reject moving a folder into itself / its own subtree.
            let is_ancestor = dst_segs.len() > src_segs.len()
                && src_segs.iter().zip(&dst_segs).all(|(a, b)| a == b);
            let same = src_segs == dst_segs;
            if same || is_ancestor {
                return Err(CkError::new(
                    codes::TYPE_MISMATCH,
                    "cannot move a folder into itself or its subtree",
                ));
            }
        }

        // Detach the source node.
        let src_parent = src_segs[..src_segs.len() - 1].join("/");
        let src_name = &src_segs[src_segs.len() - 1];
        let parent = self.find_by_path_mut(&src_parent).ok_or_else(|| {
            CkError::new(codes::PARENT_NOT_FOUND, format!("parent {src_parent:?} missing"))
        })?;
        let TreeNode::Folder { children, .. } = parent else {
            return Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("parent {src_parent:?} is not a folder"),
            ));
        };
        let idx = children
            .iter()
            .position(|c| c.name() == src_name)
            .ok_or_else(|| CkError::new(codes::NOT_FOUND, format!("{src} not found")))?;
        let node = if copy {
            children[idx].clone()
        } else {
            children.remove(idx)
        };

        // Create destination parent chain and pick a free name.
        let dst_parent = dst_segs[..dst_segs.len() - 1].join("/");
        if !dst_parent.is_empty() {
            self.ensure_folder_path(&dst_parent)?;
        }
        let base_name = &dst_segs[dst_segs.len() - 1];
        let free_name = self.available_name(&dst_parent, base_name)?;

        let mut node = node;
        // Rename the moved node to the final (possibly suffixed) name.
        {
            let name_ref: &mut String = match &mut node {
                TreeNode::Folder { name, .. } => name,
                TreeNode::Material { name, .. } => name,
            };
            *name_ref = free_name.clone();
        }
        let parent = self.find_by_path_mut(&dst_parent).ok_or_else(|| {
            CkError::new(codes::PARENT_NOT_FOUND, format!("parent {dst_parent:?} missing"))
        })?;
        let TreeNode::Folder { children, .. } = parent else {
            return Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("parent {dst_parent:?} is not a folder"),
            ));
        };
        children.push(node);

        let mut final_path = dst_segs[..dst_segs.len() - 1].to_vec();
        final_path.push(free_name);
        Ok(final_path.join("/"))
    }

    // ---- Public operations ---------------------------------------------

    /// Create a folder at `path`, auto-creating missing parents. If a node
    /// already occupies the final name, a ` (n)` suffix is used instead.
    /// Returns the actual created path.
    pub fn create_folder(&mut self, path: &str) -> Result<String, CkError> {
        let segs = parse_path(path)?;
        if segs.is_empty() {
            return Err(CkError::new(codes::INVALID_PATH, "empty path"));
        }
        let parent_path = segs[..segs.len() - 1].join("/");
        if !parent_path.is_empty() {
            self.ensure_folder_path(&parent_path)?;
        }
        let base = segs[segs.len() - 1].clone();
        let free = self.available_name(&parent_path, &base)?;
        let parent_node = self.find_by_path_mut(&parent_path).ok_or_else(|| {
            CkError::new(codes::PARENT_NOT_FOUND, format!("parent {parent_path:?} missing"))
        })?;
        let TreeNode::Folder { children, .. } = parent_node else {
            return Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("parent {parent_path:?} is not a folder"),
            ));
        };
        children.push(TreeNode::Folder {
            name: free.clone(),
            children: Vec::new(),
        });
        Ok(if parent_path.is_empty() {
            free
        } else {
            format!("{parent_path}/{free}")
        })
    }

    /// Create a material at `path` with the given file locations.
    /// fingerprint is left `None` here (the resolver computes it). Missing
    /// parents are auto-created; name collisions get a ` (n)` suffix.
    /// Returns the actual created path.
    pub fn create_material(
        &mut self,
        path: &str,
        path_abs: &str,
        path_rel: Option<&str>,
    ) -> Result<String, CkError> {
        let segs = parse_path(path)?;
        if segs.is_empty() {
            return Err(CkError::new(codes::INVALID_PATH, "empty path"));
        }
        let parent_path = segs[..segs.len() - 1].join("/");
        if !parent_path.is_empty() {
            self.ensure_folder_path(&parent_path)?;
        }
        let base = segs[segs.len() - 1].clone();
        let free = self.available_name(&parent_path, &base)?;
        let parent_node = self.find_by_path_mut(&parent_path).ok_or_else(|| {
            CkError::new(codes::PARENT_NOT_FOUND, format!("parent {parent_path:?} missing"))
        })?;
        let TreeNode::Folder { children, .. } = parent_node else {
            return Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("parent {parent_path:?} is not a folder"),
            ));
        };
        children.push(TreeNode::Material {
            name: free.clone(),
            path_abs: path_abs.to_string(),
            path_rel: path_rel.map(str::to_string),
            fingerprint: None,
        });
        Ok(if parent_path.is_empty() {
            free
        } else {
            format!("{parent_path}/{free}")
        })
    }

    /// Remove a folder at `path` (recursively over the tree only).
    pub fn remove_folder(&mut self, path: &str) -> Result<(), CkError> {
        let segs = parse_path(path)?;
        if segs.is_empty() {
            return Err(CkError::new(codes::NOT_FOUND, "root not removable"));
        }
        let parent_path = segs[..segs.len() - 1].join("/");
        let name = segs[segs.len() - 1].clone();
        let parent = self.find_by_path_mut(&parent_path).ok_or_else(|| {
            CkError::new(codes::PARENT_NOT_FOUND, format!("parent {parent_path:?} missing"))
        })?;
        let TreeNode::Folder { children, .. } = parent else {
            return Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("parent {parent_path:?} is not a folder"),
            ));
        };
        let idx = children
            .iter()
            .position(|c| c.name() == name)
            .ok_or_else(|| CkError::new(codes::NOT_FOUND, format!("{path} not found")))?;
        if children[idx].is_folder() {
            children.remove(idx);
            Ok(())
        } else {
            Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("{path} is not a folder"),
            ))
        }
    }

    /// Remove a material at `path`.
    pub fn remove_material(&mut self, path: &str) -> Result<(), CkError> {
        let segs = parse_path(path)?;
        if segs.is_empty() {
            return Err(CkError::new(codes::NOT_FOUND, "root not removable"));
        }
        let parent_path = segs[..segs.len() - 1].join("/");
        let name = segs[segs.len() - 1].clone();
        let parent = self.find_by_path_mut(&parent_path).ok_or_else(|| {
            CkError::new(codes::PARENT_NOT_FOUND, format!("parent {parent_path:?} missing"))
        })?;
        let TreeNode::Folder { children, .. } = parent else {
            return Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("parent {parent_path:?} is not a folder"),
            ));
        };
        let idx = children
            .iter()
            .position(|c| c.name() == name)
            .ok_or_else(|| CkError::new(codes::NOT_FOUND, format!("{path} not found")))?;
        if children[idx].is_material() {
            children.remove(idx);
            Ok(())
        } else {
            Err(CkError::new(
                codes::TYPE_MISMATCH,
                format!("{path} is not a material"),
            ))
        }
    }

    /// Rename a node. `new_rel` is a path relative to the node's parent:
    /// either a plain new name, or one embedding new folders (auto-created).
    /// Returns the final full path of the node.
    pub fn rename(&mut self, path: &str, new_rel: &str) -> Result<String, CkError> {
        let segs = parse_path(path)?;
        let new_segs = parse_path(new_rel)?;
        if segs.is_empty() || new_segs.is_empty() {
            return Err(CkError::new(codes::INVALID_PATH, "empty path in rename"));
        }
        let parent_path = segs[..segs.len() - 1].join("/");

        // Resolve the (possibly new) destination parent relative to the
        // source parent.
        let mut dst_segs: Vec<String> = if parent_path.is_empty() {
            Vec::new()
        } else {
            parse_path(&parent_path)?
        };
        dst_segs.extend(new_segs);
        let dst_path = dst_segs.join("/");

        self.transfer(path, &dst_path, false)
    }

    /// Copy a node (folder recurses) from `src` to `dst` (full path).
    pub fn copy(&mut self, src: &str, dst: &str) -> Result<String, CkError> {
        self.transfer(src, dst, true)
    }

    /// Move a node from `src` to `dst` (full path).
    pub fn move_(&mut self, src: &str, dst: &str) -> Result<String, CkError> {
        self.transfer(src, dst, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err_code<T: std::fmt::Debug>(r: Result<T, CkError>) -> &'static str {
        r.unwrap_err().code
    }

    #[test]
    fn create_folder_auto_builds_parent_chain() {
        let mut tree = MaterialTree::new();
        let p = tree.create_folder("songs/op").unwrap();
        assert_eq!(p, "songs/op");
        assert!(tree.find_by_path("songs").unwrap().is_folder());
        assert!(tree.find_by_path("songs/op").unwrap().is_folder());
    }

    #[test]
    fn create_folder_same_name_gets_suffix() {
        let mut tree = MaterialTree::new();
        tree.create_folder("songs").unwrap();
        let p = tree.create_folder("songs").unwrap();
        assert_eq!(p, "songs (1)");
        assert!(tree.contains_path("songs"));
        assert!(tree.contains_path("songs (1)"));
    }

    #[test]
    fn create_material_suffix_before_extension() {
        let mut tree = MaterialTree::new();
        tree.create_material("audio/song.mp3", "C:/x/song.mp3", None).unwrap();
        let p = tree.create_material("audio/song.mp3", "C:/x/song.mp3", None).unwrap();
        assert_eq!(p, "audio/song (1).mp3");
        let node = tree.find_by_path("audio/song (1).mp3").unwrap();
        assert!(node.is_material());
        assert_eq!(node.name(), "song (1).mp3");
    }

    #[test]
    fn invalid_paths_rejected() {
        let mut tree = MaterialTree::new();
        assert_eq!(err_code(tree.create_folder("a/../b")), codes::INVALID_PATH);
        assert_eq!(err_code(tree.create_folder("a/./b")), codes::INVALID_PATH);
        assert_eq!(err_code(tree.create_folder("a//b")), codes::INVALID_PATH);
    }

    #[test]
    fn remove_folder_recursive_and_type_checked() {
        let mut tree = MaterialTree::new();
        tree.create_folder("a").unwrap();
        tree.create_material("a/b.mp3", "C:/x/b.mp3", None).unwrap();
        assert_eq!(
            err_code(tree.remove_folder("a/b.mp3")),
            codes::TYPE_MISMATCH
        );
        tree.remove_folder("a").unwrap();
        assert!(!tree.contains_path("a"));
        assert_eq!(err_code(tree.remove_folder("zz")), codes::NOT_FOUND);
    }

    #[test]
    fn remove_material_type_checked() {
        let mut tree = MaterialTree::new();
        tree.create_folder("a").unwrap();
        assert_eq!(err_code(tree.remove_material("a")), codes::TYPE_MISMATCH);
    }

    #[test]
    fn rename_plain_and_with_new_subpath() {
        let mut tree = MaterialTree::new();
        tree.create_material("a/c.txt", "C:/x/c.txt", None).unwrap();
        let p = tree.rename("a/c.txt", "c1.txt").unwrap();
        assert_eq!(p, "a/c1.txt");
        let p = tree.rename("a/c1.txt", "cc/c.txt").unwrap();
        assert_eq!(p, "a/cc/c.txt");
        assert!(tree.contains_path("a/cc/c.txt"));
        assert!(!tree.contains_path("a/c1.txt"));
    }

    #[test]
    fn rename_dotdot_rejected() {
        let mut tree = MaterialTree::new();
        tree.create_material("a/c.txt", "C:/x/c.txt", None).unwrap();
        assert_eq!(
            err_code(tree.rename("a/c.txt", "../evil.txt")),
            codes::INVALID_PATH
        );
    }

    #[test]
    fn copy_clones_node_and_file_fields() {
        let mut tree = MaterialTree::new();
        tree.create_material("a/f.mp3", "C:/x/f.mp3", None).unwrap();
        if let Some(TreeNode::Material { fingerprint, .. }) = tree.find_by_path_mut("a/f.mp3") {
            *fingerprint = Some("fp1".to_string());
        }
        let p = tree.copy("a/f.mp3", "b/f.mp3").unwrap();
        assert_eq!(p, "b/f.mp3");
        assert!(tree.contains_path("a/f.mp3"));
        let TreeNode::Material { fingerprint, .. } = tree.find_by_path("b/f.mp3").unwrap() else {
            panic!("not a material");
        };
        assert_eq!(fingerprint.as_deref(), Some("fp1"));
    }

    #[test]
    fn move_removes_source() {
        let mut tree = MaterialTree::new();
        tree.create_material("a/f.mp3", "C:/x/f.mp3", None).unwrap();
        let p = tree.move_("a/f.mp3", "b/f.mp3").unwrap();
        assert_eq!(p, "b/f.mp3");
        assert!(!tree.contains_path("a/f.mp3"));
        assert!(tree.contains_path("b/f.mp3"));
    }

    #[test]
    fn move_folder_into_itself_rejected() {
        let mut tree = MaterialTree::new();
        tree.create_folder("a").unwrap();
        tree.create_folder("a/sub").unwrap();
        assert_eq!(
            err_code(tree.move_("a", "a/sub/deeper")),
            codes::TYPE_MISMATCH
        );
    }

    #[test]
    fn copy_folder_recurses() {
        let mut tree = MaterialTree::new();
        tree.create_folder("src").unwrap();
        tree.create_material("src/inner/x.mp3", "C:/x/x.mp3", None).unwrap();
        let p = tree.copy("src", "dst").unwrap();
        assert_eq!(p, "dst");
        assert!(tree.contains_path("dst/inner/x.mp3"));
        assert!(tree.contains_path("src/inner/x.mp3"));
    }
}
