use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{AddStage, App, PromptKind};

/// Two-step reminder entry popup (text, then optional due date).
pub fn reminder(frame: &mut Frame, area: Rect, app: &App) {
    let (label, value) = match app.add_stage {
        AddStage::Text => ("reminder text:", &app.input_text),
        AddStage::Due => ("due date (YYYY-MM-DD, empty = none):", &app.input_due),
    };
    draw(frame, area, "add reminder", label, value);
}

/// Single-line prompt popup for creating a note/folder or renaming a note.
pub fn prompt(frame: &mut Frame, area: Rect, app: &App) {
    let (title, label) = match app.prompt_kind {
        PromptKind::NewNote => ("new note", "filename:"),
        PromptKind::NewFolder => ("new folder", "folder name:"),
        PromptKind::Rename => ("rename note", "new name:"),
    };
    draw(frame, area, title, label, &app.prompt_input);
}

/// Yes/no delete confirmation popup.
pub fn confirm(frame: &mut Frame, area: Rect, app: &App) {
    let name = app
        .delete_target
        .as_deref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("?");
    let what = if app.delete_is_dir {
        format!("delete folder '{name}' and ALL its contents?")
    } else {
        format!("delete '{name}'?")
    };
    let popup = centered(area, 60, 5);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(format!("{what}\n\n[y] confirm   [any] cancel")).block(
            Block::default()
                .borders(Borders::ALL)
                .title("confirm delete"),
        ),
        popup,
    );
}

fn draw(frame: &mut Frame, area: Rect, title: &str, label: &str, value: &str) {
    let popup = centered(area, 60, 7);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(format!("{label}\n\n> {value}_")).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string()),
        ),
        popup,
    );
}

/// A `width`x`height` rectangle centered within `area`.
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let cols = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(area);
    Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(cols[0])[0]
}
