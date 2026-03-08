use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;

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
