use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use crate::Cli;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_start_review;
use crate::requests::send_thread_compact;
use crate::requests::send_turn_interrupt;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::state::thread_id;

pub(crate) fn try_handle_thread_action_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    _resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "compact" => {
            if require_idle_turn(state, output)? {
                let current_thread_id = thread_id(state)?.to_string();
                output.line_stderr("[thread] requesting compaction")?;
                send_thread_compact(writer, state, current_thread_id)?;
            }
            true
        }
        "review" => {
            if require_idle_turn(state, output)? {
                let current_thread_id = thread_id(state)?.to_string();
                let trimmed_args = args.join(" ");
                let trimmed_args = trimmed_args.trim();
                let (target, description) = if trimmed_args.is_empty() {
                    (
                        json!({"type": "uncommittedChanges"}),
                        "current uncommitted changes".to_string(),
                    )
                } else {
                    (
                        json!({"type": "custom", "instructions": trimmed_args}),
                        trimmed_args.to_string(),
                    )
                };
                output.line_stderr(format!(
                    "[review] requesting {}",
                    summarize_text(&description)
                ))?;
                send_start_review(writer, state, current_thread_id, target, description)?;
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
        _ => return Ok(None),
    };

    Ok(Some(result))
}
