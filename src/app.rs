use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{Local, NaiveDate};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::editor;
use crate::fuzzy;
use crate::models::Reminder;
use crate::notes::{NoteTree, TreeNode};
use crate::store::ReminderStore;
use crate::ui;

const PREVIEW_LINES: usize = 200;
const DATE_FMT: &str = "%Y-%m-%d";

#[derive(PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    AddReminder,
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

/// One rendered row of the left panel (tree node or search hit).
pub struct TreeRow {
    pub text: String,
    pub is_dir: bool,
}

/// Whole-application state. Field visibility is `pub` where the `ui` module
/// needs read access; mutation stays behind the methods below.
pub struct App {
    pub notes_dir: PathBuf,
    tree: NoteTree,
    visible: Vec<TreeNode>,
    files: Vec<PathBuf>,
    file_labels: Vec<String>,
    results: Vec<usize>,
    pub selected: usize,
    pub search: String,
    store: ReminderStore,
    pub reminders: Vec<Reminder>,
    pub rem_selected: usize,
    pub mode: Mode,
    pub focus: Focus,
    pub add_stage: AddStage,
    pub input_text: String,
    pub input_due: String,
    pub preview: String,
    pub status: String,
    should_quit: bool,
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
            store,
            reminders: Vec::new(),
            rem_selected: 0,
            mode: Mode::Normal,
            focus: Focus::Tree,
            add_stage: AddStage::Text,
            input_text: String::new(),
            input_due: String::new(),
            preview: String::new(),
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
                .filter_map(|&i| self.file_labels.get(i))
                .map(|label| TreeRow {
                    text: label.clone(),
                    is_dir: false,
                })
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

    // ---- event dispatch ------------------------------------------------------

    fn handle_event(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let Event::Key(key) = event::read()? else {
            return Ok(());
        };
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        match self.mode {
            Mode::Normal => self.on_normal_key(key, terminal)?,
            Mode::Search => self.on_search_key(key),
            Mode::AddReminder => self.on_add_key(key)?,
        }
        Ok(())
    }

    fn on_normal_key(&mut self, key: KeyEvent, terminal: &mut DefaultTerminal) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Char('/') => self.enter_search(),
            KeyCode::Char('a') => self.enter_add(),
            KeyCode::Char('d') => self.dismiss_reminder()?,
            KeyCode::Char('e') => self.edit_selected(terminal)?,
            KeyCode::Char('r') => self.reload()?,
            KeyCode::Enter | KeyCode::Char('l') => self.activate(terminal)?,
            _ => {}
        }
        Ok(())
    }

    fn on_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.exit_search(),
            KeyCode::Enter => self.mode = Mode::Normal,
            KeyCode::Down => self.move_down(),
            KeyCode::Up => self.move_up(),
            KeyCode::Backspace => {
                self.search.pop();
                self.recompute_search();
            }
            KeyCode::Char(c) => {
                self.search.push(c);
                self.recompute_search();
            }
            _ => {}
        }
    }

    fn on_add_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => self.cancel_add(),
            KeyCode::Enter => self.advance_add()?,
            KeyCode::Backspace => {
                self.active_input().pop();
            }
            KeyCode::Char(c) => self.active_input().push(c),
            _ => {}
        }
        Ok(())
    }

    // ---- navigation ----------------------------------------------------------

    fn move_down(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected + 1 < self.list_len() {
                    self.selected += 1;
                    self.update_preview();
                }
            }
            Focus::Reminders => {
                if self.rem_selected + 1 < self.reminders.len() {
                    self.rem_selected += 1;
                }
            }
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Focus::Tree => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.update_preview();
                }
            }
            Focus::Reminders => self.rem_selected = self.rem_selected.saturating_sub(1),
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tree => Focus::Reminders,
            Focus::Reminders => Focus::Tree,
        };
    }

    // ---- activation / editing ------------------------------------------------

    fn activate(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        if self.is_searching() {
            return self.edit_selected(terminal);
        }
        let Some(node) = self.visible.get(self.selected) else {
            return Ok(());
        };
        if node.is_dir {
            let path = node.path.clone();
            self.tree.toggle(&path);
            self.refresh_tree();
        } else {
            self.edit_selected(terminal)?;
        }
        Ok(())
    }

    fn edit_selected(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let Some(path) = self.selected_file() else {
            return Ok(());
        };
        editor::edit_in_neovim(&path)?;
        terminal.clear()?;
        self.reload()?;
        Ok(())
    }

    fn selected_file(&self) -> Option<PathBuf> {
        if self.is_searching() {
            let idx = *self.results.get(self.selected)?;
            return self.files.get(idx).cloned();
        }
        let node = self.visible.get(self.selected)?;
        (!node.is_dir).then(|| node.path.clone())
    }

    // ---- search --------------------------------------------------------------

    fn enter_search(&mut self) {
        self.mode = Mode::Search;
        self.focus = Focus::Tree;
        self.selected = 0;
    }

    fn exit_search(&mut self) {
        self.search.clear();
        self.results.clear();
        self.mode = Mode::Normal;
        self.selected = 0;
        self.update_preview();
    }

    fn recompute_search(&mut self) {
        self.results = fuzzy::filter(&self.search, &self.file_labels);
        if self.selected >= self.list_len() {
            self.selected = 0;
        }
        self.update_preview();
    }

    // ---- reminders -----------------------------------------------------------

    fn enter_add(&mut self) {
        self.mode = Mode::AddReminder;
        self.add_stage = AddStage::Text;
        self.input_text.clear();
        self.input_due.clear();
        self.status.clear();
    }

    fn cancel_add(&mut self) {
        self.mode = Mode::Normal;
        self.status.clear();
    }

    fn active_input(&mut self) -> &mut String {
        match self.add_stage {
            AddStage::Text => &mut self.input_text,
            AddStage::Due => &mut self.input_due,
        }
    }

    fn advance_add(&mut self) -> Result<()> {
        if self.add_stage == AddStage::Text {
            if self.input_text.trim().is_empty() {
                self.status = "reminder text is empty".into();
                return Ok(());
            }
            self.add_stage = AddStage::Due;
            return Ok(());
        }
        self.save_reminder()
    }

    fn save_reminder(&mut self) -> Result<()> {
        let due = match parse_due(&self.input_due) {
            Ok(due) => due,
            Err(message) => {
                self.status = message;
                return Ok(());
            }
        };
        self.store
            .add(self.input_text.trim(), due, Local::now().timestamp())?;
        self.mode = Mode::Normal;
        self.status = "reminder added".into();
        self.refresh_reminders()
    }

    fn dismiss_reminder(&mut self) -> Result<()> {
        let Some(reminder) = self.reminders.get(self.rem_selected) else {
            return Ok(());
        };
        self.store.dismiss(reminder.id, Local::now().timestamp())?;
        self.refresh_reminders()?;
        self.status = "reminder dismissed".into();
        Ok(())
    }

    // ---- data refresh --------------------------------------------------------

    fn reload(&mut self) -> Result<()> {
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

    fn refresh_reminders(&mut self) -> Result<()> {
        self.reminders = self.store.active()?;
        if self.rem_selected >= self.reminders.len() {
            self.rem_selected = self.reminders.len().saturating_sub(1);
        }
        Ok(())
    }

    fn update_preview(&mut self) {
        self.preview = match self.selected_file() {
            Some(path) => read_preview(&path),
            None => String::new(),
        };
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
fn parse_due(input: &str) -> Result<Option<NaiveDate>, String> {
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
