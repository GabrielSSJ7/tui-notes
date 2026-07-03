use chrono::Local;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::{App, Focus};
use crate::models::Reminder;
use crate::ui::focus_style;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let today = Local::now().date_naive();
    let items: Vec<ListItem> = app
        .reminders
        .iter()
        .map(|reminder| ListItem::new(reminder_line(reminder, reminder.is_overdue(today))))
        .collect();

    let focused = app.focus == Focus::Reminders;
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("reminders")
                .border_style(focus_style(focused)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    if focused && !app.reminders.is_empty() {
        state.select(Some(app.rem_selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn reminder_line(reminder: &Reminder, overdue: bool) -> Line<'static> {
    let mut spans = vec![Span::raw(format!("• {}", reminder.text))];
    if let Some(due) = reminder.due {
        let (label, color) = if overdue {
            (format!("  OVERDUE {due}"), Color::Red)
        } else {
            (format!("  [{due}]"), Color::Green)
        };
        spans.push(Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}
