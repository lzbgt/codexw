use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;

use crate::render_markdown_code::render_code_block;
use crate::render_markdown_inline::render_inline_markdown;
use crate::render_markdown_inline::tint_spans;

pub(crate) fn render_markdown_text(body: &str) -> Text<'static> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_language = String::new();
    let mut code_buffer = String::new();

    for raw_line in body.lines() {
        let trimmed = raw_line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            if in_code_block {
                lines.extend(render_code_block(&code_language, &code_buffer).lines);
                code_buffer.clear();
                code_language.clear();
                in_code_block = false;
            } else {
                code_language = rest.trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_buffer.push_str(raw_line);
            code_buffer.push('\n');
            continue;
        }

        if trimmed.is_empty() {
            lines.push(Line::default());
            continue;
        }

        if let Some((level, content)) = parse_heading(trimmed) {
            lines.push(Line::from(vec![
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
            continue;
        }

        if trimmed.starts_with('>') {
            let quote = trimmed.trim_start_matches('>').trim_start();
            let mut spans = vec![Span::styled("▏ ", Style::default().fg(Color::Green))];
            spans.extend(tint_spans(render_inline_markdown(quote), Color::Green));
            lines.push(Line::from(spans));
            continue;
        }

        if let Some(content) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            let mut spans = vec![Span::raw("• ")];
            spans.extend(render_inline_markdown(content));
            lines.push(Line::from(spans));
            continue;
        }

        if let Some((marker, content)) = parse_numbered_list(trimmed) {
            let mut spans = vec![Span::styled(
                format!("{marker} "),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(render_inline_markdown(content));
            lines.push(Line::from(spans));
            continue;
        }

        if trimmed.chars().all(|ch| ch == '-' || ch == '—') && trimmed.len() >= 3 {
            lines.push(Line::from(Span::styled(
                "────────────────",
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }

        lines.push(Line::from(render_inline_markdown(raw_line)));
    }

    if in_code_block {
        lines.extend(render_code_block(&code_language, &code_buffer).lines);
    }

    Text::from(lines)
}

pub(crate) fn tint_text(mut text: Text<'static>, color: Color) -> Text<'static> {
    for line in &mut text.lines {
        line.spans = tint_spans(line.spans.clone(), color);
    }
    text
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
