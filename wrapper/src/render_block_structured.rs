use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;

use crate::render_block_common::split_kv_label;
use crate::render_block_common::style_status_line;
use crate::render_markdown_code::highlight_code_line;
use crate::render_markdown_inline::render_inline_markdown;

pub(crate) fn render_diff_text(body: &str) -> Text<'static> {
    let lines = body
        .lines()
        .map(|line| {
            let style = if line.starts_with("diff --git")
                || line.starts_with("+++ ")
                || line.starts_with("--- ")
            {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else if line.starts_with("@@") {
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD)
            } else if line.starts_with('+') {
                Style::default().fg(Color::Green)
            } else if line.starts_with('-') {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(line.to_string(), style))
        })
        .collect::<Vec<_>>();
    Text::from(lines)
}

pub(crate) fn render_command_text(body: &str) -> Text<'static> {
    let lines = body
        .lines()
        .map(|line| {
            if line.starts_with("$ ") {
                Line::from(vec![
                    Span::styled(
                        "$ ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        line.trim_start_matches("$ ").to_string(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if line.starts_with("[cwd] ")
                || line.starts_with("[status] ")
                || line.starts_with("[exit] ")
                || line.starts_with("[stdout]")
                || line.starts_with("[stderr]")
            {
                style_status_line(line)
            } else if let Some((label, value)) = split_kv_label(line) {
                Line::from(vec![
                    Span::styled(
                        label.to_string(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(value.to_string()),
                ])
            } else {
                highlight_code_line("shell", line)
            }
        })
        .collect::<Vec<_>>();
    Text::from(lines)
}

pub(crate) fn render_plain_text(body: &str) -> Text<'static> {
    let lines = body
        .lines()
        .map(|line| {
            if let Some((label, value)) = split_kv_label(line) {
                Line::from(vec![
                    Span::styled(
                        label.to_string(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(value.to_string()),
                ])
            } else {
                Line::from(render_inline_markdown(line))
            }
        })
        .collect::<Vec<_>>();
    Text::from(lines)
}
