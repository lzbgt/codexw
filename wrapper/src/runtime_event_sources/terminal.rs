use std::io::Write;

use anyhow::Context;
use anyhow::Result;
use crossterm::event::DisableBracketedPaste;
use crossterm::event::EnableBracketedPaste;
use crossterm::execute;
use crossterm::terminal;

pub(crate) struct RawModeGuard;

impl RawModeGuard {
    pub(crate) fn new() -> Result<Self> {
        reset_terminal_charset()?;
        terminal::enable_raw_mode().context("enable raw terminal mode")?;
        set_bracketed_paste(true)?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = set_bracketed_paste(false);
        let _ = reset_terminal_charset();
        let _ = terminal::disable_raw_mode();
    }
}

fn set_bracketed_paste(enabled: bool) -> Result<()> {
    let mut stderr = std::io::stderr();
    if enabled {
        execute!(stderr, EnableBracketedPaste).context("enable bracketed paste")?;
    } else {
        execute!(stderr, DisableBracketedPaste).context("disable bracketed paste")?;
    }
    stderr.flush().context("flush bracketed paste mode")
}

fn reset_terminal_charset() -> Result<()> {
    const TEXT_CHARSET_RESET: &str = "\x0f\x1b(B\x1b)B";
    let mut stderr = std::io::stderr();
    write!(stderr, "{TEXT_CHARSET_RESET}").context("reset terminal character set")?;
    stderr.flush().context("flush terminal character set reset")
}
