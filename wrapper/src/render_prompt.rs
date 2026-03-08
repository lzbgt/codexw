use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use unicode_width::UnicodeWidthStr;

use crate::render_ansi::text_to_ansi;
pub(crate) use crate::render_prompt_commit::render_committed_prompt;
use crate::render_prompt_layout::fit_prompt_buffer;
use crate::render_prompt_layout::preview_prompt_buffer;

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
