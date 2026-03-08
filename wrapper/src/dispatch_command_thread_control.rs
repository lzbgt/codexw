use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_thread_compact;
use crate::requests::send_turn_interrupt;
use crate::state::AppState;
use crate::state::thread_id;

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_handle_thread_control_command(
    command: &str,
    _args: &[&str],
    cli: &Cli,
    _resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let handled = match command {
        "compact" => {
            if require_idle_turn(state, output)? {
                let current_thread_id = thread_id(state)?.to_string();
                output.line_stderr("[thread] requesting compaction")?;
                send_thread_compact(writer, state, current_thread_id)?;
            }
            true
        }
        "clean" => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[thread] background terminal cleanup requires experimental API support; restart without --no-experimental-api",
                )?;
            } else {
                let current_thread_id = thread_id(state)?.to_string();
                output.line_stderr("[thread] cleaning background terminals")?;
                send_clean_background_terminals(writer, state, current_thread_id)?;
            }
            true
        }
        "interrupt" => {
            if let Some(turn_id) = state.active_turn_id.clone() {
                output.line_stderr("[interrupt] interrupting active turn")?;
                send_turn_interrupt(writer, state, thread_id(state)?.to_string(), turn_id)?;
            } else if let Some(process_id) = state.active_exec_process_id.clone() {
                output.line_stderr("[interrupt] terminating active local command")?;
                send_command_exec_terminate(writer, state, process_id)?;
            } else {
                output.line_stderr("[interrupt] no active turn")?;
            }
            true
        }
        _ => false,
    };

    if handled { Ok(Some(true)) } else { Ok(None) }
}
