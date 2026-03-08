use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn handle_ps_command(
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("clean")) {
        if cli.no_experimental_api {
            output.line_stderr(
                "[thread] /ps clean requires experimental API support; restart without --no-experimental-api",
            )?;
        } else {
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[thread] cleaning background terminals")?;
            send_clean_background_terminals(writer, state, current_thread_id)?;
        }
    } else {
        output.line_stderr(
            "[session] app-server does not expose background-terminal listing like the native TUI; use /ps clean to stop all background terminals for this thread",
        )?;
    }
    Ok(true)
}
