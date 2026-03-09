use std::io;
use std::io::Write;

use crossterm::terminal;

use crate::output::render::render_block_lines_to_ansi;
use crate::output::render::render_line_to_ansi;
use crate::output::render::write_crlf;
use crate::render_prompt::fit_status_line;
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_lines;

const CLEAR_LINE: &str = "\r\x1b[2K";

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
