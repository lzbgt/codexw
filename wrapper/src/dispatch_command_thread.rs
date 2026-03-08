#[path = "dispatch_command_thread_flow.rs"]
mod dispatch_command_thread_flow;
#[path = "dispatch_command_thread_workspace.rs"]
mod dispatch_command_thread_workspace;

use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_thread_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) = dispatch_command_thread_flow::try_handle_thread_flow_command(
        command,
        args,
        cli,
        resolved_cwd,
        state,
        editor,
        output,
        writer,
    )? {
        return Ok(Some(result));
    }
    dispatch_command_thread_workspace::try_handle_thread_workspace_command(
        command,
        args,
        cli,
        resolved_cwd,
        state,
        editor,
        output,
        writer,
    )
}
