use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;

pub(crate) fn try_render_inline_code(
    chars: &[char],
    index: usize,
) -> Option<(Span<'static>, usize)> {
    if chars.get(index) != Some(&'`') {
        return None;
    }
    let mut end = index + 1;
    while end < chars.len() && chars[end] != '`' {
        end += 1;
    }
    if end >= chars.len() {
        return None;
    }
    let code = chars[index + 1..end].iter().collect::<String>();
    Some((
        Span::styled(code, Style::default().fg(Color::Cyan)),
        end + 1,
    ))
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

pub(crate) fn flush_plain_span(spans: &mut Vec<Span<'static>>, current: &mut String, style: Style) {
    if current.is_empty() {
        return;
    }
    spans.push(Span::styled(std::mem::take(current), style));
}

pub(crate) fn toggle_modifier(style: Style, modifier: Modifier) -> Style {
    if style.add_modifier.contains(modifier) {
        style.remove_modifier(modifier)
    } else {
        style.add_modifier(modifier)
    }
}
