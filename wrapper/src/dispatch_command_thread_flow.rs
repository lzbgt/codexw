#[path = "dispatch_command_thread_actions.rs"]
mod dispatch_command_thread_actions;
#[path = "dispatch_command_thread_navigation.rs"]
mod dispatch_command_thread_navigation;

use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_thread_flow_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) = dispatch_command_thread_navigation::try_handle_thread_navigation_command(
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
    dispatch_command_thread_actions::try_handle_thread_action_command(
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
