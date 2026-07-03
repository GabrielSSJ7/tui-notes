use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let paragraph = Paragraph::new(app.preview.clone())
        .block(Block::default().borders(Borders::ALL).title("preview"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
