use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

#[path = "dispatch_command_thread_navigation_identity.rs"]
mod dispatch_command_thread_navigation_identity;
#[path = "dispatch_command_thread_navigation_session.rs"]
mod dispatch_command_thread_navigation_session;

pub(crate) fn try_handle_thread_navigation_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) =
        dispatch_command_thread_navigation_session::try_handle_thread_session_navigation(
            command,
            args,
            cli,
            resolved_cwd,
            state,
            output,
            writer,
        )?
    {
        return Ok(Some(result));
    }
    dispatch_command_thread_navigation_identity::try_handle_thread_identity_navigation(
        command,
        args,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
    )
}
