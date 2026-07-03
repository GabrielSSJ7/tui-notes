use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::md;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let body: Text = if app.preview_is_md {
        md::render(&app.preview)
    } else {
        Text::from(app.preview.clone())
    };
    let paragraph = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title("preview"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
