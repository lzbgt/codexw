use std::sync::LazyLock;

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::render_ansi::line_to_ansi;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static SYNTAX_THEME: LazyLock<Theme> = LazyLock::new(|| {
    let themes = ThemeSet::load_defaults();
    themes
        .themes
        .get("base16-ocean.dark")
        .cloned()
        .unwrap_or_default()
});

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BlockKind {
    Markdown,
    Diff,
    Command,
    Thinking,
    Plain,
}

pub(crate) fn render_block_lines_to_ansi(title: &str, body: &str) -> Vec<String> {
    let mut text = Text::default();
    text.lines.push(render_title_line(title));
    if !body.trim().is_empty() {
        text.lines.push(Line::default());
        text.lines.extend(match classify_block(title, body) {
            BlockKind::Markdown => render_markdown_text(body).lines,
            BlockKind::Diff => render_diff_text(body).lines,
            BlockKind::Command => render_command_text(body).lines,
            BlockKind::Thinking => tint_text(render_markdown_text(body), Color::DarkGray).lines,
            BlockKind::Plain => render_plain_text(body).lines,
        });
    }
    text.lines.iter().map(line_to_ansi).collect()
}

pub(crate) fn render_line_to_ansi(line: &str) -> String {
    if line.trim().is_empty() {
        return String::new();
    }
    line_to_ansi(&style_status_line(line))
}

fn classify_block(title: &str, body: &str) -> BlockKind {
    let title = title.to_ascii_lowercase();
    if title.contains("assistant") {
        BlockKind::Markdown
    } else if title.contains("thinking") {
        BlockKind::Thinking
    } else if title.contains("diff")
        || title.contains("file change")
        || body.lines().any(|line| {
            line.starts_with("diff --git ")
                || line.starts_with("@@")
                || line.starts_with("+++ ")
                || line.starts_with("--- ")
        })
    {
        BlockKind::Diff
    } else if title.contains("command") || title.contains("$ command") {
        BlockKind::Command
    } else {
        BlockKind::Plain
    }
}

fn render_title_line(title: &str) -> Line<'static> {
    let accent = if title.eq_ignore_ascii_case("Assistant") {
        Color::Blue
    } else if title.eq_ignore_ascii_case("Thinking") {
        Color::DarkGray
    } else if title.to_ascii_lowercase().contains("diff") {
        Color::Blue
    } else if title.to_ascii_lowercase().contains("command") {
        Color::Green
    } else {
        Color::DarkGray
    };
    Line::from(vec![
        Span::styled("▌ ", Style::default().fg(accent)),
        Span::styled(
            title.to_string(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn render_markdown_text(body: &str) -> Text<'static> {
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

fn render_diff_text(body: &str) -> Text<'static> {
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

fn render_command_text(body: &str) -> Text<'static> {
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
                Line::from(style_status_line(line))
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

fn render_plain_text(body: &str) -> Text<'static> {
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

fn render_code_block(language: &str, code: &str) -> Text<'static> {
    let syntax = syntax_for_language(language);
    let mut highlighter = HighlightLines::new(syntax, &SYNTAX_THEME);
    let mut lines = Vec::new();
    if !language.trim().is_empty() {
        lines.push(Line::from(Span::styled(
            format!("```{}", language.trim()),
            Style::default().fg(Color::DarkGray),
        )));
    }
    for line in LinesWithEndings::from(code) {
        match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(ranges) => {
                let spans = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        Span::styled(
                            text.replace('\n', ""),
                            Style::default()
                                .fg(Color::Rgb(
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                ))
                                .bg(Color::Rgb(
                                    style.background.r,
                                    style.background.g,
                                    style.background.b,
                                )),
                        )
                    })
                    .collect::<Vec<_>>();
                lines.push(Line::from(spans));
            }
            Err(_) => lines.push(Line::from(Span::raw(
                line.trim_end_matches('\n').to_string(),
            ))),
        }
    }
    if !language.trim().is_empty() {
        lines.push(Line::from(Span::styled(
            "```",
            Style::default().fg(Color::DarkGray),
        )));
    }
    Text::from(lines)
}

fn syntax_for_language(language: &str) -> &'static SyntaxReference {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        return SYNTAX_SET.find_syntax_plain_text();
    }
    SYNTAX_SET
        .find_syntax_by_token(trimmed)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension(trimmed))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
}

fn highlight_code_line(language: &str, line: &str) -> Line<'static> {
    let syntax = syntax_for_language(language);
    let mut highlighter = HighlightLines::new(syntax, &SYNTAX_THEME);
    match highlighter.highlight_line(line, &SYNTAX_SET) {
        Ok(ranges) => Line::from(
            ranges
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(
                        text.to_string(),
                        Style::default().fg(Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        )),
                    )
                })
                .collect::<Vec<_>>(),
        ),
        Err(_) => Line::from(Span::raw(line.to_string())),
    }
}

fn render_inline_markdown(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut style = Style::default();

    while index < chars.len() {
        if index + 1 < chars.len() && chars[index] == '*' && chars[index + 1] == '*' {
            flush_plain_span(&mut spans, &mut current, style);
            style = toggle_modifier(style, Modifier::BOLD);
            index += 2;
            continue;
        }
        if index + 1 < chars.len() && chars[index] == '~' && chars[index + 1] == '~' {
            flush_plain_span(&mut spans, &mut current, style);
            style = toggle_modifier(style, Modifier::CROSSED_OUT);
            index += 2;
            continue;
        }
        if chars[index] == '*' {
            flush_plain_span(&mut spans, &mut current, style);
            style = toggle_modifier(style, Modifier::ITALIC);
            index += 1;
            continue;
        }
        if chars[index] == '`' {
            flush_plain_span(&mut spans, &mut current, style);
            let mut end = index + 1;
            while end < chars.len() && chars[end] != '`' {
                end += 1;
            }
            if end < chars.len() {
                let code = chars[index + 1..end].iter().collect::<String>();
                spans.push(Span::styled(code, Style::default().fg(Color::Cyan)));
                index = end + 1;
                continue;
            }
        }
        if chars[index] == '['
            && let Some((label_end, url_end)) = markdown_link_bounds(&chars, index)
        {
            flush_plain_span(&mut spans, &mut current, style);
            let label = chars[index + 1..label_end].iter().collect::<String>();
            let url = chars[label_end + 2..url_end].iter().collect::<String>();
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED),
            ));
            if !url.starts_with('/')
                && !url.starts_with("./")
                && !url.starts_with("../")
                && !url.starts_with("app://")
                && !url.starts_with("plugin://")
            {
                spans.push(Span::styled(
                    format!(" ({url})"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            index = url_end + 1;
            continue;
        }
        current.push(chars[index]);
        index += 1;
    }

    flush_plain_span(&mut spans, &mut current, style);
    spans
}

fn markdown_link_bounds(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let mut label_end = start + 1;
    while label_end < chars.len() && chars[label_end] != ']' {
        label_end += 1;
    }
    if label_end + 1 >= chars.len() || chars.get(label_end + 1) != Some(&'(') {
        return None;
    }
    let mut url_end = label_end + 2;
    while url_end < chars.len() && chars[url_end] != ')' {
        url_end += 1;
    }
    if url_end >= chars.len() {
        return None;
    }
    Some((label_end, url_end))
}

fn flush_plain_span(spans: &mut Vec<Span<'static>>, current: &mut String, style: Style) {
    if current.is_empty() {
        return;
    }
    spans.push(Span::styled(std::mem::take(current), style));
}

fn toggle_modifier(style: Style, modifier: Modifier) -> Style {
    if style.add_modifier.contains(modifier) {
        style.remove_modifier(modifier)
    } else {
        style.add_modifier(modifier)
    }
}

fn tint_text(mut text: Text<'static>, color: Color) -> Text<'static> {
    for line in &mut text.lines {
        line.spans = tint_spans(line.spans.clone(), color);
    }
    text
}

fn tint_spans(spans: Vec<Span<'static>>, color: Color) -> Vec<Span<'static>> {
    spans
        .into_iter()
        .map(|span| {
            let style = if span.style.fg.is_none() {
                span.style.fg(color)
            } else {
                span.style
            };
            Span::styled(span.content, style)
        })
        .collect()
}

fn style_status_line(line: &str) -> Line<'static> {
    if let Some((tag, rest)) = parse_bracket_tag(line) {
        Line::from(vec![
            Span::styled(
                format!("[{tag}] "),
                Style::default()
                    .fg(match tag {
                        "session" => Color::DarkGray,
                        "interrupt" => Color::Yellow,
                        "turn-error" | "server-error" => Color::Red,
                        "approval" => Color::Magenta,
                        "ready" => Color::Green,
                        _ => Color::Blue,
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_string()),
        ])
    } else {
        Line::from(Span::raw(line.to_string()))
    }
}

fn parse_bracket_tag(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('[')?;
    let (tag, rest) = rest.split_once(']')?;
    Some((tag, rest.trim_start()))
}

fn split_kv_label(line: &str) -> Option<(&str, &str)> {
    let (label, value) = line.split_once("  ")?;
    Some((label.trim_end(), value))
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
