use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::NaiveDate;
use ratatui::DefaultTerminal;

use crate::content;
use crate::fuzzy;
use crate::models::Reminder;
use crate::notes::{self, NoteTree, TreeNode};
use crate::store::ReminderStore;
use crate::ui;

const PREVIEW_LINES: usize = 200;
const DATE_FMT: &str = "%Y-%m-%d";

#[derive(PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    AddReminder,
    Prompt,
    Confirm,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    Tree,
    Reminders,
}

/// Two-step reminder entry: text first, then optional due date.
#[derive(PartialEq, Eq)]
pub enum AddStage {
    Text,
    Due,
}

/// Whether `/` search filters by filename (fuzzy) or file content (substring).
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SearchScope {
    Name,
    Content,
}

/// Which single-line prompt is active in `Mode::Prompt`.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PromptKind {
    NewNote,
    NewFolder,
    Rename,
}

/// A search result: index into `files`, plus a content snippet when the match
/// came from file content rather than the name.
pub struct SearchHit {
    pub file: usize,
    pub snippet: Option<String>,
}

/// One rendered row of the left panel (tree node or search hit).
pub struct TreeRow {
    pub text: String,
    pub is_dir: bool,
}

/// Whole-application state. Fields are `pub(crate)` so `ui` can read them and
/// `input` can drive transitions; construction and data refresh live here.
pub struct App {
    pub notes_dir: PathBuf,
    pub(crate) tree: NoteTree,
    pub(crate) visible: Vec<TreeNode>,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) file_labels: Vec<String>,
    pub(crate) results: Vec<SearchHit>,
    pub selected: usize,
    pub search: String,
    pub search_scope: SearchScope,
    pub(crate) store: ReminderStore,
    pub reminders: Vec<Reminder>,
    pub rem_selected: usize,
    pub mode: Mode,
    pub focus: Focus,
    pub add_stage: AddStage,
    pub input_text: String,
    pub input_due: String,
    pub prompt_kind: PromptKind,
    pub prompt_input: String,
    pub(crate) prompt_target: Option<PathBuf>,
    pub(crate) delete_target: Option<PathBuf>,
    pub preview: String,
    pub preview_is_md: bool,
    pub status: String,
    pub(crate) should_quit: bool,
}

impl App {
    pub fn new(notes_dir: PathBuf) -> Result<Self> {
        let store = ReminderStore::open(&notes_dir.join("reminders.db"))?;
        let tree = NoteTree::new(notes_dir.clone());
        let mut app = Self {
            notes_dir,
            tree,
            visible: Vec::new(),
            files: Vec::new(),
            file_labels: Vec::new(),
            results: Vec::new(),
            selected: 0,
            search: String::new(),
            search_scope: SearchScope::Name,
            store,
            reminders: Vec::new(),
            rem_selected: 0,
            mode: Mode::Normal,
            focus: Focus::Tree,
            add_stage: AddStage::Text,
            input_text: String::new(),
            input_due: String::new(),
            prompt_kind: PromptKind::NewNote,
            prompt_input: String::new(),
            prompt_target: None,
            delete_target: None,
            preview: String::new(),
            preview_is_md: false,
            status: String::new(),
            should_quit: false,
        };
        app.reload()?;
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::render(frame, self))?;
            self.handle_event(terminal)?;
        }
        Ok(())
    }

    // ---- rendering accessors -------------------------------------------------

    pub fn is_searching(&self) -> bool {
        !self.search.is_empty()
    }

    pub fn list_len(&self) -> usize {
        if self.is_searching() {
            self.results.len()
        } else {
            self.visible.len()
        }
    }

    /// Rows for the left panel: search hits when filtering, else the tree.
    pub fn display_rows(&self) -> Vec<TreeRow> {
        if self.is_searching() {
            return self
                .results
                .iter()
                .filter_map(|hit| self.hit_row(hit))
                .collect();
        }
        self.visible
            .iter()
            .map(|node| TreeRow {
                text: self.tree_label(node),
                is_dir: node.is_dir,
            })
            .collect()
    }

    fn hit_row(&self, hit: &SearchHit) -> Option<TreeRow> {
        let label = self.file_labels.get(hit.file)?;
        let text = match &hit.snippet {
            Some(snippet) => format!("{label}  — {snippet}"),
            None => label.clone(),
        };
        Some(TreeRow {
            text,
            is_dir: false,
        })
    }

    fn tree_label(&self, node: &TreeNode) -> String {
        let indent = "  ".repeat(node.depth);
        let name = node
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        if node.is_dir {
            let icon = if self.tree.is_expanded(&node.path) {
                "▾"
            } else {
                "▸"
            };
            format!("{indent}{icon} {name}/")
        } else {
            format!("{indent}  {name}")
        }
    }

    // ---- selection / preview -------------------------------------------------

    pub(crate) fn selected_file(&self) -> Option<PathBuf> {
        if self.is_searching() {
            let hit = self.results.get(self.selected)?;
            return self.files.get(hit.file).cloned();
        }
        let node = self.visible.get(self.selected)?;
        (!node.is_dir).then(|| node.path.clone())
    }

    pub(crate) fn update_preview(&mut self) {
        match self.selected_file() {
            Some(path) => {
                self.preview_is_md = notes::is_markdown(&path);
                self.preview = read_preview(&path);
            }
            None => {
                self.preview.clear();
                self.preview_is_md = false;
            }
        }
    }

    // ---- search --------------------------------------------------------------

    pub(crate) fn recompute_search(&mut self) {
        self.results = match self.search_scope {
            SearchScope::Name => fuzzy::filter(&self.search, &self.file_labels)
                .into_iter()
                .map(|file| SearchHit {
                    file,
                    snippet: None,
                })
                .collect(),
            SearchScope::Content => content::search(&self.search, &self.files)
                .into_iter()
                .map(|hit| SearchHit {
                    file: hit.file,
                    snippet: Some(hit.snippet),
                })
                .collect(),
        };
        if self.selected >= self.list_len() {
            self.selected = 0;
        }
        self.update_preview();
    }

    // ---- data refresh --------------------------------------------------------

    pub(crate) fn reload(&mut self) -> Result<()> {
        self.refresh_tree();
        self.refresh_files();
        self.recompute_search();
        self.refresh_reminders()?;
        self.update_preview();
        Ok(())
    }

    fn refresh_tree(&mut self) {
        self.visible = self.tree.visible();
        if self.selected >= self.visible.len() {
            self.selected = self.visible.len().saturating_sub(1);
        }
    }

    fn refresh_files(&mut self) {
        self.files = self.tree.all_files();
        let root = self.tree.root().to_path_buf();
        self.file_labels = self
            .files
            .iter()
            .map(|p| relative_label(&root, p))
            .collect();
    }

    pub(crate) fn refresh_reminders(&mut self) -> Result<()> {
        self.reminders = self.store.active()?;
        if self.rem_selected >= self.reminders.len() {
            self.rem_selected = self.reminders.len().saturating_sub(1);
        }
        Ok(())
    }
}

/// Path relative to the notes root, forward-slashed for display.
fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

/// First `PREVIEW_LINES` of a note, or an inline error marker.
fn read_preview(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text
            .lines()
            .take(PREVIEW_LINES)
            .collect::<Vec<_>>()
            .join("\n"),
        Err(err) => format!("<cannot read {}: {err}>", path.display()),
    }
}

/// Parse the due-date field: empty is `None`; otherwise `YYYY-MM-DD`.
pub(crate) fn parse_due(input: &str) -> Result<Option<NaiveDate>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(trimmed, DATE_FMT)
        .map(Some)
        .map_err(|_| format!("invalid date '{trimmed}', expected YYYY-MM-DD"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_due_empty_is_none() {
        assert_eq!(parse_due("   "), Ok(None));
    }

    #[test]
    fn parse_due_valid() {
        assert_eq!(
            parse_due("2026-07-04"),
            Ok(NaiveDate::from_ymd_opt(2026, 7, 4))
        );
    }

    #[test]
    fn parse_due_invalid_reports_value() {
        let err = parse_due("07/04/2026").unwrap_err();
        assert!(err.contains("07/04/2026"));
    }

    #[test]
    fn relative_label_strips_root() {
        let label = relative_label(Path::new("/notes"), Path::new("/notes/a/b.md"));
        assert_eq!(label, "a/b.md");
    }
}
