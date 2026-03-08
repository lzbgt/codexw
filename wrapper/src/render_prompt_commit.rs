use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;

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
