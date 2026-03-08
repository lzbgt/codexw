use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;

pub(crate) fn try_render_markdown_link(
    chars: &[char],
    index: usize,
) -> Option<(Vec<Span<'static>>, usize)> {
    if chars.get(index) != Some(&'[') {
        return None;
    }
    let (label_end, url_end) = markdown_link_bounds(chars, index)?;
    let label = chars[index + 1..label_end].iter().collect::<String>();
    let url = chars[label_end + 2..url_end].iter().collect::<String>();
    let mut spans = vec![Span::styled(
        label,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::UNDERLINED),
    )];
    if !is_internalish_url(&url) {
        spans.push(Span::styled(
            format!(" ({url})"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    Some((spans, url_end + 1))
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

fn is_internalish_url(url: &str) -> bool {
    url.starts_with('/')
        || url.starts_with("./")
        || url.starts_with("../")
        || url.starts_with("app://")
        || url.starts_with("plugin://")
}
