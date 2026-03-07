use std::io;
use std::io::Write;

const CLEAR_LINE: &str = "\r\x1b[2K";

#[derive(Default)]
pub struct Output {
    prompt: Option<String>,
    prompt_visible: bool,
}

impl Output {
    pub fn set_prompt(&mut self, prompt: Option<String>) {
        self.prompt = prompt;
    }

    pub fn show_prompt(&mut self, buffer: &str, cursor_chars: usize) -> io::Result<()> {
        if self.prompt.is_none() {
            self.hide_prompt()?;
            return Ok(());
        }
        self.redraw_prompt(buffer, cursor_chars)
    }

    pub fn commit_prompt(&mut self, buffer: &str) -> io::Result<()> {
        if let Some(prompt) = self.prompt.as_deref() {
            let mut stderr = io::stderr();
            write!(stderr, "{CLEAR_LINE}{prompt}{buffer}\n")?;
            stderr.flush()?;
        }
        self.prompt_visible = false;
        Ok(())
    }

    pub fn line_stdout(&mut self, line: impl AsRef<str>) -> io::Result<()> {
        self.prepare_for_output()?;
        let mut stdout = io::stdout();
        writeln!(stdout, "{}", line.as_ref())?;
        stdout.flush()?;
        Ok(())
    }

    pub fn line_stderr(&mut self, line: impl AsRef<str>) -> io::Result<()> {
        self.prepare_for_output()?;
        let mut stderr = io::stderr();
        writeln!(stderr, "{}", line.as_ref())?;
        stderr.flush()?;
        Ok(())
    }

    pub fn block_stdout(&mut self, title: &str, body: &str) -> io::Result<()> {
        self.prepare_for_output()?;
        let mut stdout = io::stdout();
        writeln!(stdout)?;
        writeln!(stdout, "{title}")?;
        if !body.trim().is_empty() {
            let body = body.trim_end_matches('\n');
            writeln!(stdout, "{body}")?;
        }
        stdout.flush()?;
        Ok(())
    }

    pub fn finish_stream(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn redraw_prompt(&mut self, buffer: &str, cursor_chars: usize) -> io::Result<()> {
        let prompt = self.prompt.as_deref().unwrap_or("");
        let cursor_byte = char_to_byte_index(buffer, cursor_chars);
        let prefix = &buffer[..cursor_byte];
        let mut stderr = io::stderr();
        write!(stderr, "{CLEAR_LINE}{prompt}{buffer}\x1b[K")?;
        write!(stderr, "\r{prompt}{prefix}")?;
        stderr.flush()?;
        self.prompt_visible = true;
        Ok(())
    }

    fn prepare_for_output(&mut self) -> io::Result<()> {
        self.hide_prompt()?;
        Ok(())
    }

    fn hide_prompt(&mut self) -> io::Result<()> {
        if self.prompt_visible {
            let mut stderr = io::stderr();
            write!(stderr, "{CLEAR_LINE}")?;
            stderr.flush()?;
            self.prompt_visible = false;
        }
        Ok(())
    }
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}
