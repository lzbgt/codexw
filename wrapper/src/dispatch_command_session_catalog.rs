use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_session_catalog_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) =
        crate::dispatch_command_session_catalog_lists::try_handle_session_catalog_list_command(
            command, state, output, writer,
        )?
    {
        return Ok(Some(result));
    }
    crate::dispatch_command_session_catalog_models::try_handle_session_catalog_model_command(
        command, args, cli, state, output, writer,
    )
}
