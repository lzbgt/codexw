use ratatui::text::Text;

use crate::render_ansi::line_to_ansi;
use crate::render_block_common::BlockKind;
use crate::render_block_common::classify_block;
use crate::render_block_common::render_title_line;
use crate::render_block_common::style_status_line;
use crate::render_block_markdown::render_markdown_text;
use crate::render_block_markdown::tint_text;
use crate::render_block_structured::render_command_text;
use crate::render_block_structured::render_diff_text;
use crate::render_block_structured::render_plain_text;

pub(crate) fn render_block_lines_to_ansi(title: &str, body: &str) -> Vec<String> {
    let mut text = Text::default();
    text.lines.push(render_title_line(title));
    if !body.trim().is_empty() {
        text.lines.push(ratatui::text::Line::default());
        text.lines.extend(match classify_block(title, body) {
            BlockKind::Markdown => render_markdown_text(body).lines,
            BlockKind::Diff => render_diff_text(body).lines,
            BlockKind::Command => render_command_text(body).lines,
            BlockKind::Thinking => {
                tint_text(render_markdown_text(body), ratatui::style::Color::DarkGray).lines
            }
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
