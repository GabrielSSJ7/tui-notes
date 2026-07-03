mod add_popup;
mod footer;
mod preview;
mod reminders;
mod search;
mod tree;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::Frame;

use crate::app::{App, Mode};

/// Draw the full layout: search + tree (left), reminders + preview (right),
/// a footer, and the add-reminder popup when active.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let cols = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer[0]);
    let left = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(cols[0]);
    let right =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(cols[1]);

    search::render(frame, left[0], app);
    tree::render(frame, left[1], app);
    reminders::render(frame, right[0], app);
    preview::render(frame, right[1], app);
    footer::render(frame, outer[1], app);

    if app.mode == Mode::AddReminder {
        add_popup::render(frame, area, app);
    }
}

/// Border color signalling which panel currently has focus.
pub(crate) fn focus_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}
