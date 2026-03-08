use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use crate::Cli;
use crate::dispatch_command_utils::join_prompt;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_list_threads;
use crate::requests::send_start_review;
use crate::requests::send_thread_compact;
use crate::requests::send_thread_fork;
use crate::requests::send_thread_rename;
use crate::requests::send_thread_resume;
use crate::requests::send_thread_start;
use crate::requests::send_turn_interrupt;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::state::thread_id;

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
    let result = match command {
        "new" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                true
            } else {
                output.line_stderr("[session] creating new thread")?;
                send_thread_start(writer, state, cli, resolved_cwd, None)?;
                true
            }
        }
        "resume" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                true
            } else if let Some(first_arg) = args.first() {
                let thread_id = if let Ok(index) = first_arg.parse::<usize>() {
                    match state.last_listed_thread_ids.get(index.saturating_sub(1)) {
                        Some(thread_id) => thread_id.clone(),
                        None => {
                            output.line_stderr("[session] no cached thread at that index; run /threads or /resume first")?;
                            return Ok(Some(true));
                        }
                    }
                } else {
                    (*first_arg).to_string()
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
                true
            } else {
                output.line_stderr(
                    "[session] loading recent threads; use /resume <n> or /resume <thread-id>",
                )?;
                send_list_threads(writer, state, resolved_cwd, None)?;
                true
            }
        }
        "fork" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                true
            } else {
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
                true
            }
        }
        "compact" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                true
            } else {
                let current_thread_id = thread_id(state)?.to_string();
                output.line_stderr("[thread] requesting compaction")?;
                send_thread_compact(writer, state, current_thread_id)?;
                true
            }
        }
        "review" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                true
            } else {
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
                true
            }
        }
        "clean" => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[thread] background terminal cleanup requires experimental API support; restart without --no-experimental-api",
                )?;
                true
            } else {
                let current_thread_id = thread_id(state)?.to_string();
                output.line_stderr("[thread] cleaning background terminals")?;
                send_clean_background_terminals(writer, state, current_thread_id)?;
                true
            }
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
            send_list_threads(writer, state, resolved_cwd, search_term)?;
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

    Ok(Some(result))
}
