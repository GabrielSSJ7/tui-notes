use std::io::{self, Stdout};
use std::path::Path;
use std::process::Command;

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};

/// Suspend the TUI, open `path` in the configured editor, then restore the
/// alternate screen. The editor is `$TUI_NOTES_EDITOR` if set, else `nvim`;
/// the override exists for power users and for driving integration tests with
/// a non-interactive stand-in.
///
/// The caller must clear/redraw afterwards, since the editor leaves the screen
/// in an arbitrary state.
pub fn open(path: &Path) -> io::Result<()> {
    let editor = std::env::var("TUI_NOTES_EDITOR").unwrap_or_else(|_| "nvim".to_string());
    let mut out: Stdout = io::stdout();
    disable_raw_mode()?;
    execute!(out, LeaveAlternateScreen)?;

    let status = Command::new(&editor).arg(path).status();

    enable_raw_mode()?;
    execute!(out, EnterAlternateScreen)?;
    status?;
    Ok(())
}
