use ratatui::style::Color;
use ratatui::text::Text;

use crate::render_markdown_block_structures::render_markdown_line;
use crate::render_markdown_code::render_code_block;
use crate::render_markdown_inline::tint_spans;

pub(crate) fn render_markdown_text(body: &str) -> Text<'static> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_language = String::new();
    let mut code_buffer = String::new();

    for raw_line in body.lines() {
        let trimmed = raw_line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            if in_code_block {
                lines.extend(render_code_block(&code_language, &code_buffer).lines);
                code_buffer.clear();
                code_language.clear();
                in_code_block = false;
            } else {
                code_language = rest.trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_buffer.push_str(raw_line);
            code_buffer.push('\n');
            continue;
        }

        if let Some(line) = render_markdown_line(raw_line, trimmed) {
            lines.push(line);
        }
    }

    if in_code_block {
        lines.extend(render_code_block(&code_language, &code_buffer).lines);
    }

    Text::from(lines)
}

pub(crate) fn tint_text(mut text: Text<'static>, color: Color) -> Text<'static> {
    for line in &mut text.lines {
        line.spans = tint_spans(line.spans.clone(), color);
    }
    text
}
