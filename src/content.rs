use std::path::{Path, PathBuf};

/// One content-search hit: index into the caller's file list and the first
/// matching line, trimmed for display.
pub struct ContentHit {
    pub file: usize,
    pub snippet: String,
}

/// Case-insensitive substring search over file contents. Unreadable files are
/// skipped. Hits keep the input order of `files`.
pub fn search(query: &str, files: &[PathBuf]) -> Vec<ContentHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    files
        .iter()
        .enumerate()
        .filter_map(|(file, path)| {
            first_match(path, &needle).map(|snippet| ContentHit { file, snippet })
        })
        .collect()
}

fn first_match(path: &Path, needle: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    match_in(&text, needle)
}

/// First line of `text` containing the already-lowercased `needle`, trimmed
/// and capped at 80 chars for display.
fn match_in(text: &str, needle: &str) -> Option<String> {
    text.lines()
        .find(|line| line.to_lowercase().contains(needle))
        .map(|line| line.trim().chars().take(80).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_in_finds_line_case_insensitive() {
        let hit = match_in("alpha\nBeta Gamma\n", "beta");
        assert_eq!(hit.as_deref(), Some("Beta Gamma"));
    }

    #[test]
    fn match_in_none_when_absent() {
        assert_eq!(match_in("alpha\nbeta\n", "zeta"), None);
    }
}
