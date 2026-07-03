use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

/// Minimal markdown-to-ratatui renderer for the preview pane: headings,
/// bullets, blockquotes, and fenced code blocks. Inline emphasis is left as
/// plain text — this is a reader hint, not a full CommonMark implementation.
pub fn render(source: &str) -> Text<'static> {
    let mut in_code = false;
    let lines: Vec<Line> = source
        .lines()
        .map(|raw| {
            if raw.trim_start().starts_with("```") {
                in_code = !in_code;
                return fence_line();
            }
            if in_code {
                return code_line(raw);
            }
            render_line(raw)
        })
        .collect();
    Text::from(lines)
}

fn render_line(raw: &str) -> Line<'static> {
    let trimmed = raw.trim_start();
    if let Some(level) = heading_level(trimmed) {
        return heading(trimmed, level);
    }
    if let Some(rest) = bullet(trimmed) {
        return Line::from(format!("  • {rest}"));
    }
    if let Some(rest) = trimmed.strip_prefix("> ") {
        return Line::from(Span::styled(
            format!("┃ {rest}"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    Line::from(raw.to_string())
}

/// `Some(n)` when the line is an `n`-level ATX heading (`#`..`######` + space).
fn heading_level(line: &str) -> Option<usize> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        Some(hashes)
    } else {
        None
    }
}

fn heading(line: &str, level: usize) -> Line<'static> {
    let text = line.trim_start_matches('#').trim().to_string();
    let color = if level <= 1 {
        Color::Magenta
    } else {
        Color::Cyan
    };
    Line::from(Span::styled(
        text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

fn bullet(line: &str) -> Option<String> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .map(str::to_string)
}

fn fence_line() -> Line<'static> {
    Line::from(Span::styled("┄┄┄", Style::default().fg(Color::DarkGray)))
}

fn code_line(raw: &str) -> Line<'static> {
    Line::from(Span::styled(
        raw.to_string(),
        Style::default().fg(Color::Green),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_levels() {
        assert_eq!(heading_level("# Title"), Some(1));
        assert_eq!(heading_level("### Sub"), Some(3));
        assert_eq!(heading_level("#NoSpace"), None);
        assert_eq!(heading_level("####### too many"), None);
        assert_eq!(heading_level("plain"), None);
    }

    #[test]
    fn bullets() {
        assert_eq!(bullet("- item").as_deref(), Some("item"));
        assert_eq!(bullet("* item").as_deref(), Some("item"));
        assert_eq!(bullet("item"), None);
    }
}
