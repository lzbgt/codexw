use std::process::ChildStdin;
use std::time::Instant;

use anyhow::Result;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::Cli;
use crate::policy::shell_program;
use crate::policy::turn_sandbox_policy;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_command_exec(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    command: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    let process_id = format!("codexw-cmd-{}", state.next_request_id);
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ExecCommand {
            process_id: process_id.clone(),
            command: command.clone(),
        },
    );
    state.process_output_buffers.remove(&process_id);
    state.active_exec_process_id = Some(process_id.clone());
    state.activity_started_at = Some(Instant::now());
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "command/exec",
            params: json!({
                "command": [shell_program(), "-lc", command],
                "processId": process_id,
                "cwd": resolved_cwd,
                "streamStdoutStderr": true,
                "sandboxPolicy": turn_sandbox_policy(cli, state),
            }),
        },
    )
}

pub(crate) fn send_command_exec_terminate(
    writer: &mut ChildStdin,
    state: &mut AppState,
    process_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::TerminateExecCommand {
            process_id: process_id.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "command/exec/terminate",
            params: json!({
                "processId": process_id,
            }),
        },
    )
}
