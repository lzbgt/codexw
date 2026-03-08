use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_submit_commands::try_handle_prefixed_submission;
use crate::dispatch_submit_turns::submit_turn_input;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn handle_user_input(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(true);
    }

    if let Some(result) =
        try_handle_prefixed_submission(trimmed, cli, resolved_cwd, state, editor, output, writer)?
    {
        return Ok(result);
    }

    if !submit_turn_input(trimmed, cli, resolved_cwd, state, writer)? {
        output.line_stderr("[session] nothing to submit")?;
        return Ok(true);
    }
    Ok(true)
}
