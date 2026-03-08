use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) fn handle_realtime_start(
    state: &mut AppState,
    output: &mut Output,
    prompt: &str,
) -> Result<()> {
    state.realtime_prompt = Some(prompt.to_string());
    output.line_stderr("[realtime] start requested")?;
    Ok(())
}

pub(crate) fn handle_realtime_append(output: &mut Output, text: &str) -> Result<()> {
    output.line_stderr(format!("[realtime] sent {}", summarize_text(text)))?;
    Ok(())
}

pub(crate) fn handle_realtime_stop(output: &mut Output) -> Result<()> {
    output.line_stderr("[realtime] stop requested")?;
    Ok(())
}
