use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::dispatch_command_thread_common::resolve_cached_thread_reference;
use crate::output::Output;
use crate::requests::send_list_threads;
use crate::requests::send_thread_resume;
use crate::requests::send_thread_start;
use crate::state::AppState;

pub(crate) fn try_handle_thread_session_navigation(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let handled = match command {
        "new" => {
            if require_idle_turn(state, output)? {
                output.line_stderr("[session] creating new thread")?;
                send_thread_start(writer, state, cli, resolved_cwd, None)?;
            }
            true
        }
        "resume" => {
            if !require_idle_turn(state, output)? {
                // Guard already reported the active turn.
            } else if let Some(first_arg) = args.first() {
                let Some(thread_id) = resolve_cached_thread_reference(first_arg, state, output)?
                else {
                    return Ok(Some(true));
                };
                output.line_stderr(format!("[session] resuming thread {thread_id}"))?;
                send_thread_resume(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id.to_string(),
                    None,
                )?;
            } else {
                output.line_stderr(
                    "[session] loading recent threads; use /resume <n> or /resume <thread-id>",
                )?;
                send_list_threads(writer, state, Some(resolved_cwd), None, true)?;
            }
            true
        }
        "threads" => {
            let search_term = args.join(" ");
            let search_term = search_term.trim();
            let search_term = if search_term.is_empty() {
                None
            } else {
                Some(search_term.to_string())
            };
            output.line_stderr("[session] loading recent threads")?;
            send_list_threads(writer, state, Some(resolved_cwd), search_term, false)?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(handled))
}
