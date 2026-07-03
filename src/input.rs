//! Keyboard event handling for `App`: mode-routed key dispatch plus the
//! navigation, editing, reminder, and note-CRUD transitions it drives.

use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::app::{AddStage, App, Focus, Mode, PromptKind, SearchScope};
use crate::{editor, fs_ops};

impl App {
    pub(crate) fn handle_event(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
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
            Mode::Prompt => self.on_prompt_key(key, terminal)?,
            Mode::Confirm => self.on_confirm_key(key)?,
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
            KeyCode::Char('n') => self.begin_new_note(),
            KeyCode::Char('R') => self.begin_rename(),
            KeyCode::Char('D') => self.begin_delete(),
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
            KeyCode::Tab => self.toggle_scope(),
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

    fn on_prompt_key(&mut self, key: KeyEvent, terminal: &mut DefaultTerminal) -> Result<()> {
        match key.code {
            KeyCode::Esc => self.cancel_prompt(),
            KeyCode::Enter => self.submit_prompt(terminal)?,
            KeyCode::Backspace => {
                self.prompt_input.pop();
            }
            KeyCode::Char(c) => self.prompt_input.push(c),
            _ => {}
        }
        Ok(())
    }

    fn on_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_delete()?,
            _ => {
                self.mode = Mode::Normal;
                self.delete_target = None;
            }
        }
        Ok(())
    }

    // ---- navigation ----------------------------------------------------------

    fn move_down(&mut self) {
        match self.focus {
            Focus::Tree if self.selected + 1 < self.list_len() => {
                self.selected += 1;
                self.update_preview();
            }
            Focus::Reminders if self.rem_selected + 1 < self.reminders.len() => {
                self.rem_selected += 1;
            }
            _ => {}
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Focus::Tree if self.selected > 0 => {
                self.selected -= 1;
                self.update_preview();
            }
            Focus::Reminders => self.rem_selected = self.rem_selected.saturating_sub(1),
            _ => {}
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tree => Focus::Reminders,
            Focus::Reminders => Focus::Tree,
        };
    }

    fn toggle_scope(&mut self) {
        self.search_scope = match self.search_scope {
            SearchScope::Name => SearchScope::Content,
            SearchScope::Content => SearchScope::Name,
        };
        self.recompute_search();
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
            self.reload()?;
        } else {
            self.edit_selected(terminal)?;
        }
        Ok(())
    }

    fn edit_selected(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let Some(path) = self.selected_file() else {
            return Ok(());
        };
        editor::open(&path)?;
        terminal.clear()?;
        self.reload()
    }

    // ---- search mode ---------------------------------------------------------

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
        let due = match crate::app::parse_due(&self.input_due) {
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

    // ---- note CRUD -----------------------------------------------------------

    fn begin_new_note(&mut self) {
        self.prompt_kind = PromptKind::NewNote;
        self.prompt_target = None;
        self.prompt_input.clear();
        self.status.clear();
        self.mode = Mode::Prompt;
    }

    fn begin_rename(&mut self) {
        let Some(path) = self.selected_file() else {
            self.status = "select a file to rename".into();
            return;
        };
        self.prompt_kind = PromptKind::Rename;
        self.prompt_input = file_name(&path);
        self.prompt_target = Some(path);
        self.status.clear();
        self.mode = Mode::Prompt;
    }

    fn begin_delete(&mut self) {
        match self.selected_file() {
            Some(path) => {
                self.delete_target = Some(path);
                self.mode = Mode::Confirm;
            }
            None => self.status = "select a file to delete".into(),
        }
    }

    fn cancel_prompt(&mut self) {
        self.mode = Mode::Normal;
        self.status.clear();
    }

    fn submit_prompt(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        match self.prompt_kind {
            PromptKind::NewNote => self.finish_new_note(terminal),
            PromptKind::Rename => self.finish_rename(),
        }
    }

    fn finish_new_note(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let dir = self.new_note_dir();
        match fs_ops::create_note(&dir, &self.prompt_input) {
            Ok(path) => {
                self.mode = Mode::Normal;
                editor::open(&path)?;
                terminal.clear()?;
                self.reload()?;
                self.select_path(&path);
                self.status = "note created".into();
            }
            Err(err) => self.status = err.to_string(),
        }
        Ok(())
    }

    fn finish_rename(&mut self) -> Result<()> {
        let Some(source) = self.prompt_target.clone() else {
            self.mode = Mode::Normal;
            return Ok(());
        };
        match fs_ops::rename(&source, &self.prompt_input) {
            Ok(path) => {
                self.mode = Mode::Normal;
                self.reload()?;
                self.select_path(&path);
                self.status = "renamed".into();
            }
            Err(err) => self.status = err.to_string(),
        }
        Ok(())
    }

    fn confirm_delete(&mut self) -> Result<()> {
        if let Some(path) = self.delete_target.take() {
            fs_ops::delete(&path)?;
            self.reload()?;
            self.status = "deleted".into();
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    /// Directory a new note lands in: the selected dir, the selected file's
    /// parent, or the notes root (also used while searching).
    fn new_note_dir(&self) -> PathBuf {
        if self.is_searching() {
            return self.tree.root().to_path_buf();
        }
        match self.visible.get(self.selected) {
            Some(node) if node.is_dir => node.path.clone(),
            Some(node) => node
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            None => self.tree.root().to_path_buf(),
        }
    }

    fn select_path(&mut self, path: &Path) {
        if let Some(index) = self.visible.iter().position(|node| node.path == path) {
            self.selected = index;
            self.update_preview();
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}
