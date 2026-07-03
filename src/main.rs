use std::path::PathBuf;

use anyhow::Result;
use tui_notes::app::App;
use tui_notes::cli;

fn main() -> Result<()> {
    let notes_dir = resolve_notes_dir();
    std::fs::create_dir_all(&notes_dir)?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("remind") {
        return cli::run_remind(&notes_dir, &args[1..]);
    }

    let mut app = App::new(notes_dir)?;
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}

/// Notes root: `$TUI_NOTES_DIR` if set, else `~/.local/tui-notes`.
fn resolve_notes_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("TUI_NOTES_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME").expect("HOME env var not set");
    PathBuf::from(home).join(".local/tui-notes")
}
