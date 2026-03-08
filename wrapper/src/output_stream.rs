use std::io;
use std::io::Write;

use crate::output::Output;
use crate::render::render_block_lines_to_ansi;
use crate::render::render_line_to_ansi;

impl Output {
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
}

pub(crate) fn write_crlf(writer: &mut impl Write, text: &str) -> io::Result<()> {
    let normalized = normalize_line_endings(text);
    write!(writer, "{normalized}\r\n")
}

fn normalize_line_endings(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    text.replace('\r', "\n").replace('\n', "\r\n")
}
