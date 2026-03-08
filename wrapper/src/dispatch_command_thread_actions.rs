#[path = "dispatch_command_thread_control.rs"]
mod dispatch_command_thread_control;
#[path = "dispatch_command_thread_review.rs"]
mod dispatch_command_thread_review;

use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_thread_action_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    _resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) = dispatch_command_thread_review::try_handle_thread_review_command(
        command,
        args,
        cli,
        _resolved_cwd,
        state,
        editor,
        output,
        writer,
    )? {
        return Ok(Some(result));
    }

    dispatch_command_thread_control::try_handle_thread_control_command(
        command,
        args,
        cli,
        _resolved_cwd,
        state,
        editor,
        output,
        writer,
    )
}
