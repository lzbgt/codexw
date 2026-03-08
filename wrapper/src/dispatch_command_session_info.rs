use std::process::ChildStdin;

use anyhow::Result;

#[path = "dispatch_command_session_catalog.rs"]
mod dispatch_command_session_catalog;
#[path = "dispatch_command_session_status.rs"]
mod dispatch_command_session_status;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_session_info_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) = dispatch_command_session_catalog::try_handle_session_catalog_command(
        command, args, cli, state, _editor, output, writer,
    )? {
        return Ok(Some(result));
    }
    dispatch_command_session_status::try_handle_session_status_command(
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
