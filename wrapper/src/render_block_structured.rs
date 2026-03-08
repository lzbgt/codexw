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
    let mut lines = Vec::new();
    let mut saw_command = false;
    let mut in_output = false;

    for line in body.lines() {
        if line.is_empty() {
            lines.push(Line::default());
            if saw_command {
                in_output = true;
            }
            continue;
        }

        if !saw_command {
            lines.push(render_shell_command_line(line));
            saw_command = true;
            continue;
        }

        if !in_output {
            if line.starts_with("[cwd] ")
                || line.starts_with("[status] ")
                || line.starts_with("[exit] ")
            {
                lines.push(style_status_line(line));
                continue;
            }
            if let Some((label, value)) = split_kv_label(line) {
                lines.push(Line::from(vec![
                    Span::styled(
                        label.to_string(),
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(value.to_string(), Style::default().fg(Color::Gray)),
                ]));
                continue;
            }
            in_output = true;
        }

        lines.push(render_command_output_line(line));
    }

    Text::from(lines)
}

pub(crate) fn render_plan_text(body: &str) -> Text<'static> {
    let lines = body
        .lines()
        .map(|line| {
            if let Some(step) = line.strip_prefix("✔ ") {
                Line::from(vec![
                    Span::styled(
                        "✔ ",
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        step.to_string(),
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::CROSSED_OUT | Modifier::DIM),
                    ),
                ])
            } else if let Some(step) = line.strip_prefix("□ ") {
                Line::from(vec![
                    Span::styled(
                        "□ ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        step.to_string(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if let Some(step) = line.strip_prefix("◦ ") {
                Line::from(vec![
                    Span::styled("◦ ", Style::default().fg(Color::Gray)),
                    Span::styled(step.to_string(), Style::default().fg(Color::Gray)),
                ])
            } else if line.trim().is_empty() {
                Line::default()
            } else {
                Line::from(Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ))
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

fn render_shell_command_line(line: &str) -> Line<'static> {
    let command = line.strip_prefix("$ ").unwrap_or(line);
    let mut spans = vec![Span::styled(
        "$ ",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )];
    spans.extend(highlight_code_line("bash", command).spans);
    Line::from(spans)
}

fn render_command_output_line(line: &str) -> Line<'static> {
    if line == "[stdout]" || line == "[stderr]" {
        return Line::from(Span::styled(
            line.to_string(),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(Span::styled(
        line.to_string(),
        Style::default().fg(Color::Gray),
    ))
}
