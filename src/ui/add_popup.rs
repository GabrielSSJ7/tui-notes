use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{AddStage, App};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 60, 7);
    let (label, value) = match app.add_stage {
        AddStage::Text => ("reminder text:", &app.input_text),
        AddStage::Due => ("due date (YYYY-MM-DD, empty = none):", &app.input_due),
    };
    let body = format!("{label}\n\n> {value}_");
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(body).block(Block::default().borders(Borders::ALL).title("add reminder")),
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
