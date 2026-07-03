use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Mode};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let keys = match app.mode {
        Mode::Normal => {
            "j/k move  Enter open/expand  e edit  / search  a add  d dismiss  Tab focus  q quit"
        }
        Mode::Search => "type to filter  up/down move  Enter accept  Esc clear",
        Mode::AddReminder => "type text  Enter next/save  Esc cancel",
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" {keys} "),
            Style::default().fg(Color::Black).bg(Color::Gray),
        ),
        Span::styled(
            format!("  {}", app.status),
            Style::default().fg(Color::Yellow),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
