use std::io;
use std::io::Write;

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;

use crate::render_ansi::line_to_ansi;
use crate::render_block_common::BlockHeaderStyle;
use crate::render_block_common::BlockKind;
use crate::render_block_common::classify_block;
use crate::render_block_common::header_style;
use crate::render_block_common::render_title_line;
use crate::render_block_common::style_status_line;
use crate::render_block_markdown::render_markdown_text;
use crate::render_block_markdown::tint_text;
use crate::render_block_structured::render_command_text;
use crate::render_block_structured::render_diff_text;
use crate::render_block_structured::render_plain_text;
use crate::render_block_structured::render_plan_text;

pub(crate) fn write_crlf(writer: &mut impl Write, text: &str) -> io::Result<()> {
    let normalized = normalize_line_endings(text);
    write!(writer, "{normalized}\r\n")
}

fn normalize_line_endings(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    text.replace('\r', "\n").replace('\n', "\r\n")
}

pub(crate) fn render_block_lines_to_ansi(title: &str, body: &str) -> Vec<String> {
    let mut text = Text::default();
    let header_style = header_style(title);
    if header_style != BlockHeaderStyle::Hidden {
        text.lines.push(render_title_line(title));
    }
    if !body.trim().is_empty() {
        if header_style != BlockHeaderStyle::Hidden {
            text.lines.push(Line::default());
        }
        text.lines.extend(match classify_block(title, body) {
            BlockKind::Markdown => render_markdown_text(body).lines,
            BlockKind::Diff => render_diff_text(body).lines,
            BlockKind::Command => render_command_text(body).lines,
            BlockKind::Plan => render_plan_text(body).lines,
            BlockKind::Thinking => tint_text(render_markdown_text(body), Color::Gray).lines,
            BlockKind::Plain => tint_text(render_plain_text(body), Color::Gray).lines,
        });
    }
    apply_transcript_prefix(title, &mut text);
    text.lines.iter().map(line_to_ansi).collect()
}

pub(crate) fn render_line_to_ansi(line: &str) -> String {
    if line.trim().is_empty() {
        return String::new();
    }
    line_to_ansi(&style_status_line(line))
}

fn apply_transcript_prefix(title: &str, text: &mut Text<'static>) {
    let (first_prefix, continuation_prefix) = match title.to_ascii_lowercase().as_str() {
        "assistant" => ("• ", "  "),
        "user" => ("› ", "  "),
        _ => return,
    };

    let prefix_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM);
    let lines = std::mem::take(&mut text.lines);
    text.lines = lines
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            let prefix = if idx == 0 {
                first_prefix
            } else {
                continuation_prefix
            };
            let mut spans = vec![Span::styled(prefix.to_string(), prefix_style)];
            spans.extend(line.spans);
            Line::from(spans)
        })
        .collect();
}
