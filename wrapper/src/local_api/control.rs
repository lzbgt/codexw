use std::collections::VecDeque;
use std::process::ChildStdin;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_submit_turns::submit_turn_input;
use crate::output::Output;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_turn_interrupt;
use crate::state::AppState;
use crate::state::thread_id;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LocalApiCommand {
    StartTurn { session_id: String, prompt: String },
    InterruptTurn { session_id: String },
}

pub(crate) type SharedCommandQueue = Arc<Mutex<VecDeque<LocalApiCommand>>>;

pub(crate) fn new_command_queue() -> SharedCommandQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub(crate) fn enqueue_command(queue: &SharedCommandQueue, command: LocalApiCommand) -> Result<()> {
    if let Ok(mut guard) = queue.lock() {
        guard.push_back(command);
    }
    Ok(())
}

fn drain_commands(queue: &SharedCommandQueue) -> Vec<LocalApiCommand> {
    match queue.lock() {
        Ok(mut guard) => guard.drain(..).collect(),
        Err(_) => Vec::new(),
    }
}

pub(crate) fn process_local_api_commands(
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    queue: &SharedCommandQueue,
) -> Result<()> {
    for command in drain_commands(queue) {
        match command {
            LocalApiCommand::StartTurn { session_id, prompt } => {
                match submit_turn_input(&prompt, cli, resolved_cwd, state, writer) {
                    Ok(true) => {
                        output.line_stderr(format!(
                            "[local-api] accepted queued turn for session {session_id}"
                        ))?;
                    }
                    Ok(false) => {
                        output.line_stderr(format!(
                            "[local-api] ignored empty turn submission for session {session_id}"
                        ))?;
                    }
                    Err(err) => {
                        output.line_stderr(format!(
                            "[local-api] failed to submit queued turn for session {session_id}: {err:#}"
                        ))?;
                    }
                }
            }
            LocalApiCommand::InterruptTurn { session_id } => {
                if let Some(turn_id) = state.active_turn_id.clone() {
                    match send_turn_interrupt(writer, state, thread_id(state)?.to_string(), turn_id) {
                        Ok(()) => {
                            output.line_stderr(format!(
                                "[local-api] requested turn interrupt for session {session_id}"
                            ))?;
                        }
                        Err(err) => {
                            output.line_stderr(format!(
                                "[local-api] failed to interrupt turn for session {session_id}: {err:#}"
                            ))?;
                        }
                    }
                } else if let Some(process_id) = state.active_exec_process_id.clone() {
                    match send_command_exec_terminate(writer, state, process_id) {
                        Ok(()) => {
                            output.line_stderr(format!(
                                "[local-api] requested local-command termination for session {session_id}"
                            ))?;
                        }
                        Err(err) => {
                            output.line_stderr(format!(
                                "[local-api] failed to terminate active local command for session {session_id}: {err:#}"
                            ))?;
                        }
                    }
                } else {
                    output.line_stderr(format!(
                        "[local-api] no active turn or local command to interrupt for session {session_id}"
                    ))?;
                }
            }
        }
    }
    Ok(())
}
