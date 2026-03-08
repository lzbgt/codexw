use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::render_ansi::text_to_ansi;

pub(crate) fn render_prompt_line(
    prompt_label: &str,
    buffer: &str,
    cursor_chars: usize,
    terminal_width: usize,
) -> (String, usize) {
    let prefix = if prompt_label.trim().is_empty() {
        "> ".to_string()
    } else {
        format!("{prompt_label}> ")
    };
    let prefix_width = UnicodeWidthStr::width(prefix.as_str());
    let available_chars = terminal_width.saturating_sub(prefix_width).max(1);
    let display_buffer = preview_prompt_buffer(buffer);
    let (visible_buffer, visible_cursor_chars) =
        fit_prompt_buffer(&display_buffer, cursor_chars, available_chars);
    let line = text_to_ansi(&Text::from(Line::from(vec![
        Span::styled(
            prefix,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(visible_buffer),
    ])));
    let cursor_col = prefix_width + visible_cursor_chars;
    (line, cursor_col)
}

pub(crate) fn render_committed_prompt(buffer: &str) -> String {
    let mut rendered_lines = Vec::new();
    for (idx, line) in buffer.split('\n').enumerate() {
        let prefix = if idx == 0 { "> " } else { "  " };
        rendered_lines.push(text_to_ansi(&Text::from(Line::from(vec![
            Span::styled(
                prefix,
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.to_string()),
        ]))));
    }
    rendered_lines.join("\n")
}

fn preview_prompt_buffer(buffer: &str) -> String {
    buffer
        .chars()
        .map(|ch| if ch == '\n' { '↩' } else { ch })
        .collect()
}

fn fit_prompt_buffer(buffer: &str, cursor_chars: usize, available_chars: usize) -> (String, usize) {
    let graphemes = UnicodeSegmentation::graphemes(buffer, true).collect::<Vec<_>>();
    let total_width = grapheme_slice_width(&graphemes);
    let cursor = cursor_chars.min(graphemes.len());
    if total_width <= available_chars {
        return (
            buffer.to_string(),
            grapheme_slice_width(&graphemes[..cursor]).min(available_chars),
        );
    }

    if available_chars <= 3 {
        return (".".repeat(available_chars), available_chars);
    }

    let window_width = available_chars - 3;
    let mut start = cursor;
    let mut width_before_cursor = 0;
    while start > 0 {
        let next_width = UnicodeWidthStr::width(graphemes[start - 1]);
        if width_before_cursor + next_width > window_width {
            break;
        }
        start -= 1;
        width_before_cursor += next_width;
    }

    let mut end = start;
    let mut visible_width = 0;
    while end < graphemes.len() {
        let next_width = UnicodeWidthStr::width(graphemes[end]);
        if visible_width + next_width > window_width {
            break;
        }
        visible_width += next_width;
        end += 1;
    }
    if end == start && end < graphemes.len() {
        end += 1;
    }

    let body = graphemes[start..end].concat();
    let mut visible = String::from("...");
    visible.push_str(&body);
    let cursor_in_visible = if start == 0 {
        grapheme_slice_width(&graphemes[..cursor])
    } else {
        3 + grapheme_slice_width(&graphemes[start..cursor])
    };
    (visible, cursor_in_visible.min(available_chars))
}

fn grapheme_slice_width(graphemes: &[&str]) -> usize {
    graphemes
        .iter()
        .map(|grapheme| UnicodeWidthStr::width(*grapheme))
        .sum()
}
