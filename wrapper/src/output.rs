use std::io;
use std::io::Write;

use crossterm::terminal;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;

pub(crate) const CLEAR_LINE: &str = "\r\x1b[2K";

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
use crate::render_prompt::fit_status_line;
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_lines;

#[derive(Default)]
pub struct Output {
    pub(crate) prompt: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) prompt_rows: usize,
    pub(crate) prompt_cursor_row: usize,
    pub(crate) status_rows: usize,
    last_frame: Option<PromptFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PromptFrame {
    status: Option<String>,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
}

impl Output {
    pub fn set_prompt(&mut self, prompt: Option<String>) {
        self.prompt = prompt;
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }

    pub fn show_prompt(&mut self, buffer: &str, cursor_chars: usize) -> io::Result<()> {
        if self.prompt.is_none() {
            self.hide_ui()?;
            return Ok(());
        }
        self.redraw_prompt(buffer, cursor_chars)
    }

    pub fn commit_prompt(&mut self, buffer: &str) -> io::Result<()> {
        if self.prompt.is_some() {
            self.hide_ui()?;
            let mut stderr = io::stderr();
            write_crlf(&mut stderr, &render_committed_prompt(buffer))?;
            stderr.flush()?;
        }
        self.prompt_rows = 0;
        self.prompt_cursor_row = 0;
        self.status_rows = 0;
        Ok(())
    }

    pub fn line_stdout(&mut self, line: impl AsRef<str>) -> io::Result<()> {
        self.prepare_for_output()?;
        let mut stderr = io::stderr();
        write_crlf(&mut stderr, &render_line_to_ansi(line.as_ref()))?;
        stderr.flush()?;
        Ok(())
    }

    pub fn line_stderr(&mut self, line: impl AsRef<str>) -> io::Result<()> {
        self.prepare_for_output()?;
        let mut stderr = io::stderr();
        write_crlf(&mut stderr, &render_line_to_ansi(line.as_ref()))?;
        stderr.flush()?;
        Ok(())
    }

    pub fn block_stdout(&mut self, title: &str, body: &str) -> io::Result<()> {
        self.prepare_for_output()?;
        let mut stderr = io::stderr();
        let lines = render_block_lines_to_ansi(title, body.trim_end_matches('\n'));
        write!(stderr, "\r\n")?;
        for line in lines {
            write_crlf(&mut stderr, &line)?;
        }
        stderr.flush()?;
        Ok(())
    }

    pub fn finish_stream(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        self.hide_ui()?;
        let mut stderr = io::stderr();
        write!(stderr, "\x1b[2J\x1b[H")?;
        stderr.flush()?;
        Ok(())
    }

    pub(crate) fn prepare_for_output(&mut self) -> io::Result<()> {
        self.hide_ui()?;
        Ok(())
    }

    pub(crate) fn redraw_prompt(&mut self, buffer: &str, cursor_chars: usize) -> io::Result<()> {
        let prompt = self.prompt.as_deref().unwrap_or("");
        let terminal_width = terminal::size()
            .map(|(width, _)| width as usize)
            .unwrap_or(120);
        let (lines, cursor_row, cursor_col) =
            render_prompt_lines(prompt, buffer, cursor_chars, terminal_width);
        let status_line = self
            .status
            .as_deref()
            .map(|status| fit_status_line(status, terminal_width));
        let next_frame = PromptFrame {
            status: status_line.clone(),
            lines: lines.clone(),
            cursor_row,
            cursor_col,
        };
        if (self.prompt_rows > 0 || self.status_rows > 0)
            && self.last_frame.as_ref() == Some(&next_frame)
        {
            return Ok(());
        }

        let mut stderr = io::stderr();
        if self.prompt_rows > 0 || self.status_rows > 0 {
            self.hide_ui()?;
        }
        if let Some(status) = status_line.as_deref() {
            write!(stderr, "\r{}\x1b[K\r\n", render_line_to_ansi(status))?;
            self.status_rows = 1;
        } else {
            self.status_rows = 0;
        }
        for (idx, line) in lines.iter().enumerate() {
            if idx > 0 {
                write!(stderr, "\r\n")?;
            }
            write!(stderr, "\r{line}\x1b[K")?;
        }
        let down_from_cursor = lines.len().saturating_sub(1).saturating_sub(cursor_row);
        if down_from_cursor > 0 {
            write!(stderr, "\x1b[{down_from_cursor}A")?;
        }
        write!(stderr, "\r\x1b[{cursor_col}C")?;
        stderr.flush()?;
        self.prompt_rows = lines.len();
        self.prompt_cursor_row = cursor_row;
        self.last_frame = Some(next_frame);
        Ok(())
    }

    pub(crate) fn hide_ui(&mut self) -> io::Result<()> {
        if self.prompt_rows == 0 && self.status_rows == 0 {
            return Ok(());
        }
        let mut stderr = io::stderr();
        if self.prompt_rows > 0 {
            let down = self
                .prompt_rows
                .saturating_sub(1)
                .saturating_sub(self.prompt_cursor_row);
            if down > 0 {
                write!(stderr, "\x1b[{down}B")?;
            }
            write!(stderr, "{CLEAR_LINE}")?;
            for _ in 1..self.prompt_rows {
                write!(stderr, "\x1b[1A{CLEAR_LINE}")?;
            }
        }
        if self.status_rows > 0 {
            if self.prompt_rows > 0 {
                write!(stderr, "\x1b[1A{CLEAR_LINE}")?;
            } else {
                write!(stderr, "{CLEAR_LINE}")?;
            }
        }
        write!(stderr, "\r")?;
        stderr.flush()?;
        self.prompt_rows = 0;
        self.prompt_cursor_row = 0;
        self.status_rows = 0;
        Ok(())
    }
}

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
            text.lines.push(ratatui::text::Line::default());
        }
        text.lines.extend(match classify_block(title, body) {
            BlockKind::Markdown => render_markdown_text(body).lines,
            BlockKind::Diff => render_diff_text(body).lines,
            BlockKind::Command => render_command_text(body).lines,
            BlockKind::Plan => render_plan_text(body).lines,
            BlockKind::Thinking => {
                tint_text(render_markdown_text(body), ratatui::style::Color::Gray).lines
            }
            BlockKind::Plain => {
                tint_text(render_plain_text(body), ratatui::style::Color::Gray).lines
            }
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
