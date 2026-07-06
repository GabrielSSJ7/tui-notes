use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

/// Create an empty note named `name` inside `dir`. Appends `.md` when no note
/// extension is given. Errors if the target already exists.
pub fn create_note(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = ensure_note_ext(dir.join(sanitize(name)?));
    if path.exists() {
        bail!("note already exists: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, "")?;
    Ok(path)
}

/// Create a subdirectory named `name` inside `dir`. Errors if it exists.
pub fn create_folder(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = dir.join(sanitize(name)?);
    if path.exists() {
        bail!("folder already exists: {}", path.display());
    }
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Rename `path` to `new_name` within its own directory.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let target = ensure_note_ext(parent.join(sanitize(new_name)?));
    if target.exists() {
        bail!("target already exists: {}", target.display());
    }
    std::fs::rename(path, &target)?;
    Ok(target)
}

pub fn delete(path: &Path) -> Result<()> {
    std::fs::remove_file(path)?;
    Ok(())
}

/// Reject empty names and any path traversal so notes stay under their dir.
fn sanitize(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("name is empty");
    }
    if trimmed.contains('/') || trimmed.contains("..") {
        bail!("name '{trimmed}' must not contain '/' or '..'");
    }
    Ok(trimmed.to_string())
}

/// Force a note extension: keep `.md`/`.txt`, otherwise default to `.md`.
fn ensure_note_ext(path: PathBuf) -> PathBuf {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") | Some("txt") => path,
        _ => path.with_extension("md"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn sanitize_rejects_traversal() {
        assert!(sanitize("../secret").is_err());
        assert!(sanitize("a/b").is_err());
        assert!(sanitize("   ").is_err());
    }

    #[test]
    fn ensure_ext_defaults_to_md() {
        assert_eq!(ensure_note_ext(PathBuf::from("todo")), Path::new("todo.md"));
        assert_eq!(ensure_note_ext(PathBuf::from("a.txt")), Path::new("a.txt"));
        assert_eq!(ensure_note_ext(PathBuf::from("a.md")), Path::new("a.md"));
    }

    #[test]
    fn create_folder_makes_dir_and_rejects_dup() {
        let base = std::env::temp_dir().join(format!("tui-notes-ut-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let made = create_folder(&base, "projects").unwrap();
        assert!(made.is_dir());
        assert!(create_folder(&base, "projects").is_err());

        std::fs::remove_dir_all(&base).unwrap();
    }
}
