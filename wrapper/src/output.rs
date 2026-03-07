use std::io;
use std::io::Write;

use crossterm::terminal;

use crate::render::render_block_lines_to_ansi;
use crate::render::render_committed_prompt;
use crate::render::render_line_to_ansi;
use crate::render::render_prompt_line;

const CLEAR_LINE: &str = "\r\x1b[2K";

#[derive(Default)]
pub struct Output {
    prompt: Option<String>,
    status: Option<String>,
    prompt_visible: bool,
    status_visible: bool,
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

    fn redraw_prompt(&mut self, buffer: &str, cursor_chars: usize) -> io::Result<()> {
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

    fn prepare_for_output(&mut self) -> io::Result<()> {
        self.hide_ui()?;
        Ok(())
    }

    fn hide_ui(&mut self) -> io::Result<()> {
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

fn write_crlf(writer: &mut impl Write, text: &str) -> io::Result<()> {
    let normalized = normalize_line_endings(text);
    write!(writer, "{normalized}\r\n")
}

fn normalize_line_endings(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    text.replace('\r', "\n").replace('\n', "\r\n")
}
