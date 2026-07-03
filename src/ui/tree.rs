use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::{App, Focus};
use crate::ui::focus_style;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .display_rows()
        .into_iter()
        .map(|row| {
            let style = if row.is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(row.text).style(style)
        })
        .collect();

    let title = if app.is_searching() {
        "results"
    } else {
        "notes"
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(focus_style(app.focus == Focus::Tree)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    if app.list_len() > 0 {
        state.select(Some(app.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}
