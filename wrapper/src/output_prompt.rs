use std::io;
use std::io::Write;

use crossterm::terminal;

use crate::output::CLEAR_LINE;
use crate::output::Output;
use crate::output::write_crlf;
use crate::render_blocks::render_line_to_ansi;
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_line;

impl Output {
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
