use std::collections::HashSet;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// A single row in the rendered tree: a directory or a note file at `depth`.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
}

/// Directory tree over the notes root. Directories are collapsed by default;
/// `toggle` expands/collapses and `visible` flattens the expanded tree.
pub struct NoteTree {
    root: PathBuf,
    expanded: HashSet<PathBuf>,
}

impl NoteTree {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            expanded: HashSet::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_expanded(&self, path: &Path) -> bool {
        self.expanded.contains(path)
    }

    pub fn toggle(&mut self, path: &Path) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_path_buf());
        }
    }

    /// Flatten the tree honoring the expanded set: dirs first, then files,
    /// each recursed into only when expanded.
    pub fn visible(&self) -> Vec<TreeNode> {
        let mut out = Vec::new();
        self.list_dir(&self.root, 0, &mut out);
        out
    }

    fn list_dir(&self, dir: &Path, depth: usize, out: &mut Vec<TreeNode>) {
        for entry in sorted_children(dir) {
            let path = entry.path();
            if path.is_dir() {
                out.push(TreeNode {
                    path: path.clone(),
                    depth,
                    is_dir: true,
                });
                if self.expanded.contains(&path) {
                    self.list_dir(&path, depth + 1, out);
                }
            } else if is_note_file(&path) {
                out.push(TreeNode {
                    path,
                    depth,
                    is_dir: false,
                });
            }
        }
    }

    /// Every note file under the root, for fuzzy search.
    pub fn all_files(&self) -> Vec<PathBuf> {
        WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| is_note_file(p))
            .collect()
    }
}

/// Directory children sorted dirs-first then case-insensitive name.
fn sorted_children(dir: &Path) -> Vec<DirEntry> {
    let mut entries: Vec<DirEntry> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => Vec::new(),
    };
    entries.sort_by_key(|e| {
        let path = e.path();
        let name = path
            .file_name()
            .map(|n| n.to_ascii_lowercase())
            .unwrap_or_default();
        (!path.is_dir(), name)
    });
    entries
}

/// True for `.md` and `.txt` files only.
pub fn is_note_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("txt")
    )
}

/// True for `.md` files, which the preview pane renders as markdown.
pub fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("md")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn note_file_accepts_md_and_txt() {
        assert!(is_note_file(Path::new("a.md")));
        assert!(is_note_file(Path::new("a.txt")));
    }

    #[test]
    fn note_file_rejects_others() {
        assert!(!is_note_file(Path::new("a.rs")));
        assert!(!is_note_file(Path::new("a")));
        assert!(!is_note_file(Path::new("dir/")));
    }
}
