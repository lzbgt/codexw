use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::dispatch_command_utils::join_prompt;
use crate::output::Output;
use crate::requests::send_thread_fork;
use crate::requests::send_thread_rename;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn try_handle_thread_identity_navigation(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let handled = match command {
        "fork" => {
            if require_idle_turn(state, output)? {
                let current_thread_id = thread_id(state)?.to_string();
                let initial_prompt =
                    join_prompt(&args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>());
                output.line_stderr(format!("[thread] forking {current_thread_id}"))?;
                send_thread_fork(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    current_thread_id,
                    initial_prompt,
                )?;
            }
            true
        }
        "rename" => {
            let name = args.join(" ").trim().to_string();
            if name.is_empty() {
                output.line_stderr("[session] usage: :rename <name>")?;
                return Ok(Some(true));
            }
            let current_thread_id = thread_id(state)?.to_string();
            send_thread_rename(writer, state, current_thread_id, name)?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(handled))
}
