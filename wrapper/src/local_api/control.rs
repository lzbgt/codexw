use std::collections::VecDeque;
use std::process::ChildStdin;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_submit_turns::submit_turn_input;
use crate::output::Output;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_thread_resume;
use crate::requests::send_thread_start;
use crate::requests::send_turn_interrupt;
use crate::state::AppState;
use crate::state::thread_id;

use super::SharedSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LocalApiCommand {
    StartSessionThread {
        session_id: String,
        client_id: Option<String>,
        lease_seconds: Option<u64>,
    },
    AttachSessionThread {
        session_id: String,
        thread_id: String,
        client_id: Option<String>,
        lease_seconds: Option<u64>,
    },
    StartTurn {
        session_id: String,
        prompt: String,
    },
    InterruptTurn {
        session_id: String,
    },
    StartShell {
        session_id: String,
        arguments: serde_json::Value,
    },
    SendShellInput {
        session_id: String,
        arguments: serde_json::Value,
    },
    TerminateShell {
        session_id: String,
        arguments: serde_json::Value,
    },
    UpdateService {
        session_id: String,
        arguments: serde_json::Value,
    },
    UpdateDependencies {
        session_id: String,
        arguments: serde_json::Value,
    },
    RenewAttachmentLease {
        session_id: String,
        client_id: Option<String>,
        lease_seconds: u64,
    },
    ReleaseAttachment {
        session_id: String,
        client_id: Option<String>,
    },
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
    snapshot: &SharedSnapshot,
    queue: &SharedCommandQueue,
) -> Result<()> {
    for command in drain_commands(queue) {
        match command {
            LocalApiCommand::StartSessionThread {
                session_id,
                client_id,
                lease_seconds,
            } => match send_thread_start(writer, state, cli, resolved_cwd, None) {
                Ok(()) => {
                    apply_attachment_metadata(snapshot, client_id.as_deref(), lease_seconds);
                    output.line_stderr(format!(
                        "[local-api] requested fresh thread start for session {session_id}"
                    ))?;
                }
                Err(err) => {
                    output.line_stderr(format!(
                        "[local-api] failed to start fresh thread for session {session_id}: {err:#}"
                    ))?;
                }
            },
            LocalApiCommand::AttachSessionThread {
                session_id,
                thread_id,
                client_id,
                lease_seconds,
            } => {
                match send_thread_resume(writer, state, cli, resolved_cwd, thread_id.clone(), None)
                {
                    Ok(()) => {
                        apply_attachment_metadata(snapshot, client_id.as_deref(), lease_seconds);
                        output.line_stderr(format!(
                            "[local-api] requested thread attach for session {session_id}: {thread_id}"
                        ))?;
                    }
                    Err(err) => {
                        output.line_stderr(format!(
                            "[local-api] failed to attach thread for session {session_id} ({thread_id}): {err:#}"
                        ))?;
                    }
                }
            }
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
                    match send_turn_interrupt(writer, state, thread_id(state)?.to_string(), turn_id)
                    {
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
            LocalApiCommand::StartShell {
                session_id,
                arguments,
            } => match state
                .orchestration
                .background_shells
                .start_from_tool_with_context(
                    &arguments,
                    resolved_cwd,
                    crate::background_shells::BackgroundShellOrigin {
                        source_tool: Some("local_api".to_string()),
                        ..Default::default()
                    },
                ) {
                Ok(summary) => {
                    output.line_stderr(format!(
                        "[local-api] started background shell for session {session_id}: {summary}"
                    ))?;
                }
                Err(err) => {
                    output.line_stderr(format!(
                        "[local-api] failed to start background shell for session {session_id}: {err}"
                    ))?;
                }
            },
            LocalApiCommand::SendShellInput {
                session_id,
                arguments,
            } => match state
                .orchestration
                .background_shells
                .send_input_from_tool(&arguments)
            {
                Ok(summary) => {
                    output.line_stderr(format!(
                        "[local-api] sent input to background shell for session {session_id}: {summary}"
                    ))?;
                }
                Err(err) => {
                    output.line_stderr(format!(
                        "[local-api] failed to send input to background shell for session {session_id}: {err}"
                    ))?;
                }
            },
            LocalApiCommand::TerminateShell {
                session_id,
                arguments,
            } => match state
                .orchestration
                .background_shells
                .terminate_from_tool(&arguments)
            {
                Ok(summary) => {
                    output.line_stderr(format!(
                        "[local-api] terminated background shell for session {session_id}: {summary}"
                    ))?;
                }
                Err(err) => {
                    output.line_stderr(format!(
                        "[local-api] failed to terminate background shell for session {session_id}: {err}"
                    ))?;
                }
            },
            LocalApiCommand::UpdateService {
                session_id,
                arguments,
            } => match state
                .orchestration
                .background_shells
                .update_service_from_tool(&arguments)
            {
                Ok(summary) => {
                    output.line_stderr(format!(
                        "[local-api] updated service state for session {session_id}: {summary}"
                    ))?;
                }
                Err(err) => {
                    output.line_stderr(format!(
                        "[local-api] failed to update service state for session {session_id}: {err}"
                    ))?;
                }
            },
            LocalApiCommand::UpdateDependencies {
                session_id,
                arguments,
            } => match state
                .orchestration
                .background_shells
                .update_dependencies_from_tool(&arguments)
            {
                Ok(summary) => {
                    output.line_stderr(format!(
                        "[local-api] updated dependency state for session {session_id}: {summary}"
                    ))?;
                }
                Err(err) => {
                    output.line_stderr(format!(
                        "[local-api] failed to update dependency state for session {session_id}: {err}"
                    ))?;
                }
            },
            LocalApiCommand::RenewAttachmentLease {
                session_id,
                client_id,
                lease_seconds,
            } => {
                apply_attachment_metadata(snapshot, client_id.as_deref(), Some(lease_seconds));
                output.line_stderr(format!(
                    "[local-api] renewed attachment lease for session {session_id}: {lease_seconds}s"
                ))?;
            }
            LocalApiCommand::ReleaseAttachment {
                session_id,
                client_id,
            } => {
                clear_attachment_metadata(snapshot, client_id.as_deref());
                output.line_stderr(format!(
                    "[local-api] released attachment lease for session {session_id}"
                ))?;
            }
        }
    }
    Ok(())
}

fn apply_attachment_metadata(
    snapshot: &SharedSnapshot,
    client_id: Option<&str>,
    lease_seconds: Option<u64>,
) {
    let Ok(mut guard) = snapshot.write() else {
        return;
    };
    guard.attachment_client_id = client_id.map(ToOwned::to_owned);
    guard.attachment_lease_seconds = lease_seconds;
    guard.attachment_lease_expires_at_ms = lease_seconds.and_then(lease_expiry_ms);
}

fn clear_attachment_metadata(snapshot: &SharedSnapshot, client_id: Option<&str>) {
    let Ok(mut guard) = snapshot.write() else {
        return;
    };
    if client_id.is_some() && guard.attachment_client_id.as_deref() != client_id {
        return;
    }
    guard.attachment_client_id = None;
    guard.attachment_lease_seconds = None;
    guard.attachment_lease_expires_at_ms = None;
}

fn lease_expiry_ms(seconds: u64) -> Option<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let delta = u128::from(seconds).checked_mul(1000)?;
    let expiry = now.checked_add(delta)?;
    u64::try_from(expiry).ok()
}
