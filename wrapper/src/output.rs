use std::io;
use std::io::Write;

use crossterm::terminal;
use ratatui::text::Text;

pub(crate) const CLEAR_LINE: &str = "\r\x1b[2K";

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
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_line;

#[derive(Default)]
pub struct Output {
    pub(crate) prompt: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) prompt_visible: bool,
    pub(crate) status_visible: bool,
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
        self.prompt_visible = false;
        self.status_visible = false;
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
        let (line, cursor_col) = render_prompt_line(prompt, buffer, cursor_chars, terminal_width);
        let mut stderr = io::stderr();
        if self.prompt_visible || self.status_visible {
            self.hide_ui()?;
        }
        if let Some(status) = self.status.as_deref() {
            write!(stderr, "\r{}\x1b[K\r\n", render_line_to_ansi(status))?;
            self.status_visible = true;
        } else {
            self.status_visible = false;
        }
        write!(stderr, "\r{line}\x1b[K")?;
        write!(stderr, "\r\x1b[{cursor_col}C")?;
        stderr.flush()?;
        self.prompt_visible = true;
        Ok(())
    }

    pub(crate) fn hide_ui(&mut self) -> io::Result<()> {
        if !self.prompt_visible && !self.status_visible {
            return Ok(());
        }
        let mut stderr = io::stderr();
        if self.prompt_visible {
            write!(stderr, "{CLEAR_LINE}")?;
        }
        if self.status_visible {
            write!(stderr, "\x1b[1A{CLEAR_LINE}\r")?;
        }
        stderr.flush()?;
        self.prompt_visible = false;
        self.status_visible = false;
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
