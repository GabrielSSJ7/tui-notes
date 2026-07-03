use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.mode == Mode::Search;
    let cursor = if active { "_" } else { "" };
    let style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let paragraph = Paragraph::new(format!("🔍 {}{cursor}", app.search)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("search")
            .border_style(style),
    );
    frame.render_widget(paragraph, area);
}
