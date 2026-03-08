use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::render_ansi::text_to_ansi;

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

pub(crate) fn render_prompt_lines(
    prompt_label: &str,
    buffer: &str,
    cursor_chars: usize,
    terminal_width: usize,
) -> (Vec<String>, usize, usize) {
    let prefix = if prompt_label.trim().is_empty() {
        "> ".to_string()
    } else {
        format!("{prompt_label}> ")
    };
    let prefix_width = UnicodeWidthStr::width(prefix.as_str());
    let continuation_prefix = " ".repeat(prefix_width.max(2));
    let available_chars = terminal_width.saturating_sub(prefix_width).max(1);
    let display_buffer = preview_prompt_buffer(buffer);
    let (visible_lines, cursor_row, visible_cursor_chars) =
        wrap_prompt_buffer(&display_buffer, cursor_chars, available_chars);

    let mut rendered = Vec::with_capacity(visible_lines.len());
    for (idx, visible_buffer) in visible_lines.into_iter().enumerate() {
        let prompt_prefix = if idx == 0 {
            prefix.clone()
        } else {
            continuation_prefix.clone()
        };
        rendered.push(text_to_ansi(&Text::from(Line::from(vec![
            Span::styled(
                prompt_prefix,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(visible_buffer),
        ]))));
    }
    let cursor_col = prefix_width + visible_cursor_chars;
    (rendered, cursor_row, cursor_col)
}

pub(crate) fn fit_status_line(status: &str, terminal_width: usize) -> String {
    if terminal_width == 0 {
        return String::new();
    }

    let graphemes = UnicodeSegmentation::graphemes(status, true).collect::<Vec<_>>();
    let total_width = grapheme_slice_width(&graphemes);
    if total_width <= terminal_width {
        return status.to_string();
    }

    if terminal_width <= 3 {
        return ".".repeat(terminal_width);
    }

    let mut visible = String::new();
    let mut used = 0;
    let budget = terminal_width - 3;
    for grapheme in graphemes {
        let width = UnicodeWidthStr::width(grapheme);
        if used + width > budget {
            break;
        }
        visible.push_str(grapheme);
        used += width;
    }
    visible.push_str("...");
    visible
}

fn preview_prompt_buffer(buffer: &str) -> String {
    buffer
        .chars()
        .map(|ch| if ch == '\n' { '⏎' } else { ch })
        .collect()
}

fn wrap_prompt_buffer(
    buffer: &str,
    cursor_chars: usize,
    available_chars: usize,
) -> (Vec<String>, usize, usize) {
    let graphemes = UnicodeSegmentation::graphemes(buffer, true).collect::<Vec<_>>();
    let cursor = cursor_chars.min(graphemes.len());
    let line_width = available_chars.max(1);

    if graphemes.is_empty() {
        return (vec![String::new()], 0, 0);
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut cursor_row = 0;
    let mut cursor_col = 0;

    for (idx, grapheme) in graphemes.iter().enumerate() {
        if idx == cursor {
            cursor_row = lines.len();
            cursor_col = current_width.min(line_width);
        }

        let grapheme_width = UnicodeWidthStr::width(*grapheme).max(1);
        if current_width > 0 && current_width + grapheme_width > line_width {
            lines.push(current);
            current = String::new();
            current_width = 0;
        }

        current.push_str(grapheme);
        current_width += grapheme_width;
    }

    if cursor == graphemes.len() {
        cursor_row = lines.len();
        cursor_col = current_width.min(line_width);
    }

    lines.push(current);
    (lines, cursor_row, cursor_col)
}

fn grapheme_slice_width(graphemes: &[&str]) -> usize {
    graphemes
        .iter()
        .map(|grapheme| UnicodeWidthStr::width(*grapheme))
        .sum()
}
