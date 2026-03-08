use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::render_markdown_links::try_render_markdown_link;
use crate::render_markdown_styles::flush_plain_span;
pub(crate) use crate::render_markdown_styles::tint_spans;
use crate::render_markdown_styles::toggle_modifier;
use crate::render_markdown_styles::try_render_inline_code;

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
        if let Some((span, next_index)) = try_render_inline_code(&chars, index) {
            flush_plain_span(&mut spans, &mut current, style);
            spans.push(span);
            index = next_index;
            continue;
        }
        if let Some((link_spans, next_index)) = try_render_markdown_link(&chars, index) {
            flush_plain_span(&mut spans, &mut current, style);
            spans.extend(link_spans);
            index = next_index;
            continue;
        }
        current.push(chars[index]);
        index += 1;
    }

    flush_plain_span(&mut spans, &mut current, style);
    spans
}
