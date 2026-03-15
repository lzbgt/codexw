use anyhow::Result;
use std::process::ChildStdin;

use crate::Cli;
use crate::dispatch_submit_turns::submit_turn_input;
use crate::output::Output;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_thread_resume;
use crate::requests::send_thread_start;
use crate::requests::send_turn_interrupt;
use crate::state::AppState;
use crate::state::thread_id;

use super::super::SharedSnapshot;
use super::attachment::apply_attachment_metadata;
use super::attachment::clear_attachment_metadata;
use super::queue::LocalApiCommand;
use super::queue::SharedCommandQueue;
use super::queue::drain_commands;

pub(super) fn process_local_api_commands(
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
