use std::io::{self, Stdout};
use std::path::Path;
use std::process::Command;

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};

/// Suspend the TUI, edit `path` in neovim, then restore the alternate screen.
///
/// The caller must clear/redraw the terminal afterwards, since neovim leaves
/// the screen in an arbitrary state.
pub fn edit_in_neovim(path: &Path) -> io::Result<()> {
    let mut out: Stdout = io::stdout();
    disable_raw_mode()?;
    execute!(out, LeaveAlternateScreen)?;

    let status = Command::new("nvim").arg(path).status();

    enable_raw_mode()?;
    execute!(out, EnterAlternateScreen)?;
    status?;
    Ok(())
}
