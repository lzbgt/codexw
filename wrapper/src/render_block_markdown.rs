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

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static SYNTAX_THEME: LazyLock<Theme> = LazyLock::new(|| {
    let themes = ThemeSet::load_defaults();
    themes
        .themes
        .get("base16-ocean.dark")
        .cloned()
        .unwrap_or_default()
});

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

pub(crate) fn highlight_code_line(language: &str, line: &str) -> Line<'static> {
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

pub(crate) fn render_inline_markdown(text: &str) -> Vec<Span<'static>> {
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

pub(crate) fn tint_text(mut text: Text<'static>, color: Color) -> Text<'static> {
    for line in &mut text.lines {
        line.spans = tint_spans(line.spans.clone(), color);
    }
    text
}

pub(crate) fn tint_spans(spans: Vec<Span<'static>>, color: Color) -> Vec<Span<'static>> {
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
