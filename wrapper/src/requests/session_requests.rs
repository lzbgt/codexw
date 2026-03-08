use std::process::ChildStdin;
use std::time::Instant;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::Cli;
use crate::input::ParsedInput;
use crate::policy::approval_policy;
use crate::policy::reasoning_summary;
use crate::policy::shell_program;
use crate::policy::thread_sandbox_mode;
use crate::policy::turn_sandbox_policy;
use crate::rpc::OutgoingRequest;
use crate::session::current_collaboration_mode_value;
use crate::state::AppState;

pub(crate) fn send_thread_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    initial_prompt: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending_thread_switch = true;
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartThread { initial_prompt },
    );

    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/start",
            params: json!({
                "model": cli.model,
                "modelProvider": cli.model_provider,
                "cwd": resolved_cwd,
                "approvalPolicy": approval_policy(cli),
                "sandbox": thread_sandbox_mode(cli),
                "serviceName": "codexw_terminal",
                "experimentalRawEvents": false,
            }),
        },
    )
}

pub(crate) fn send_thread_resume(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: String,
    initial_prompt: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending_thread_switch = true;
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ResumeThread { initial_prompt },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/resume",
            params: json!({
                "threadId": thread_id,
                "model": cli.model,
                "modelProvider": cli.model_provider,
                "cwd": resolved_cwd,
                "approvalPolicy": approval_policy(cli),
                "sandbox": thread_sandbox_mode(cli),
            }),
        },
    )
}

pub(crate) fn send_thread_fork(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: String,
    initial_prompt: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending_thread_switch = true;
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ForkThread { initial_prompt },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/fork",
            params: json!({
                "threadId": thread_id,
                "cwd": resolved_cwd,
                "model": cli.model,
                "modelProvider": cli.model_provider,
                "approvalPolicy": approval_policy(cli),
                "sandbox": thread_sandbox_mode(cli),
            }),
        },
    )
}

pub(crate) fn send_thread_compact(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::CompactThread);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/compact/start",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}

pub(crate) fn send_thread_rename(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    name: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::RenameThread { name: name.clone() },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/name/set",
            params: json!({
                "threadId": thread_id,
                "name": name,
            }),
        },
    )
}

pub(crate) fn send_clean_background_terminals(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::CleanBackgroundTerminals);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/backgroundTerminals/clean",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}

pub(crate) fn send_thread_realtime_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    prompt: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartRealtime {
            prompt: prompt.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/realtime/start",
            params: json!({
                "threadId": thread_id,
                "prompt": prompt,
                "sessionId": state.realtime_session_id.clone(),
            }),
        },
    )
}

pub(crate) fn send_thread_realtime_append_text(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    text: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::AppendRealtimeText { text: text.clone() },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/realtime/appendText",
            params: json!({
                "threadId": thread_id,
                "text": text,
            }),
        },
    )
}

pub(crate) fn send_thread_realtime_stop(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::StopRealtime);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/realtime/stop",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}

pub(crate) fn send_start_review(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    review_target: Value,
    target_description: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartReview { target_description },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "review/start",
            params: json!({
                "threadId": thread_id,
                "delivery": "inline",
                "target": review_target,
            }),
        },
    )
}

pub(crate) fn send_turn_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: String,
    submission: ParsedInput,
    auto_generated: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartTurn { auto_generated },
    );
    if !auto_generated && state.objective.is_none() && !submission.display_text.trim().is_empty() {
        state.objective = Some(submission.display_text.clone());
    }

    let mut params = json!({
        "threadId": thread_id,
        "input": submission.items,
        "cwd": resolved_cwd,
        "approvalPolicy": approval_policy(cli),
        "sandboxPolicy": turn_sandbox_policy(cli),
        "model": cli.model,
        "summary": reasoning_summary(cli),
    });
    if let Some(personality) = state.active_personality.as_deref() {
        params["personality"] = Value::String(personality.to_string());
    }
    if let Some(collaboration_mode) = current_collaboration_mode_value(state) {
        params["collaborationMode"] = collaboration_mode;
    }

    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/start",
            params,
        },
    )
}

pub(crate) fn send_turn_steer(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    turn_id: String,
    submission: ParsedInput,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::SteerTurn {
            display_text: submission.display_text.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/steer",
            params: json!({
                "threadId": thread_id,
                "expectedTurnId": turn_id,
                "input": submission.items,
            }),
        },
    )
}

pub(crate) fn send_turn_interrupt(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    turn_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::InterruptTurn);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/interrupt",
            params: json!({
                "threadId": thread_id,
                "turnId": turn_id,
            }),
        },
    )
}

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
                "sandboxPolicy": turn_sandbox_policy(cli),
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
