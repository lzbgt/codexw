#[path = "dispatch_command_session_meta.rs"]
mod dispatch_command_session_meta;
#[path = "dispatch_command_session_modes.rs"]
mod dispatch_command_session_modes;

use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_session_control_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    _resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) = dispatch_command_session_modes::try_handle_session_mode_command(
        command,
        args,
        cli,
        _resolved_cwd,
        state,
        _editor,
        output,
        writer,
    )? {
        return Ok(Some(result));
    }
    dispatch_command_session_meta::try_handle_session_meta_command(
        command,
        args,
        cli,
        _resolved_cwd,
        state,
        _editor,
        output,
        writer,
    )
}
