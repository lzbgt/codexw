use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::render_markdown_inline::render_inline_markdown;
use crate::render_markdown_inline::tint_spans;

pub(crate) fn render_markdown_line(raw_line: &str, trimmed: &str) -> Option<Line<'static>> {
    if trimmed.is_empty() {
        return Some(Line::default());
    }

    if let Some((level, content)) = parse_heading(trimmed) {
        return Some(Line::from(vec![
            Span::styled(
                match level {
                    1 => "# ",
                    2 => "## ",
                    3 => "### ",
                    _ => "#### ",
                },
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                content.to_string(),
                match level {
                    1 => Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    2 => Style::default().add_modifier(Modifier::BOLD),
                    3 => Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC),
                    _ => Style::default().add_modifier(Modifier::ITALIC),
                },
            ),
        ]));
    }

    if trimmed.starts_with('>') {
        let quote = trimmed.trim_start_matches('>').trim_start();
        let mut spans = vec![Span::styled("▏ ", Style::default().fg(Color::Green))];
        spans.extend(tint_spans(render_inline_markdown(quote), Color::Green));
        return Some(Line::from(spans));
    }

    if let Some(content) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        let mut spans = vec![Span::raw("• ")];
        spans.extend(render_inline_markdown(content));
        return Some(Line::from(spans));
    }

    if let Some((marker, content)) = parse_numbered_list(trimmed) {
        let mut spans = vec![Span::styled(
            format!("{marker} "),
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )];
        spans.extend(render_inline_markdown(content));
        return Some(Line::from(spans));
    }

    if trimmed.chars().all(|ch| ch == '-' || ch == '—') && trimmed.len() >= 3 {
        return Some(Line::from(Span::styled(
            "────────────────",
            Style::default().fg(Color::DarkGray),
        )));
    }

    Some(Line::from(render_inline_markdown(raw_line)))
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let hash_count = line.chars().take_while(|ch| *ch == '#').count();
    if (1..=6).contains(&hash_count) && line.chars().nth(hash_count) == Some(' ') {
        Some((hash_count, line[hash_count + 1..].trim()))
    } else {
        None
    }
}

fn parse_numbered_list(line: &str) -> Option<(&str, &str)> {
    let (marker, content) = line.split_once(". ")?;
    if marker.chars().all(|ch| ch.is_ascii_digit()) {
        Some((marker, content))
    } else {
        None
    }
}
