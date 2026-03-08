#[path = "dispatch_command_session_control.rs"]
mod dispatch_command_session_control;
#[path = "dispatch_command_session_info.rs"]
mod dispatch_command_session_info;

use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_session_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) = dispatch_command_session_info::try_handle_session_info_command(
        command,
        args,
        cli,
        resolved_cwd,
        state,
        _editor,
        output,
        writer,
    )? {
        return Ok(Some(result));
    }
    dispatch_command_session_control::try_handle_session_control_command(
        command,
        args,
        cli,
        resolved_cwd,
        state,
        _editor,
        output,
        writer,
    )
}
