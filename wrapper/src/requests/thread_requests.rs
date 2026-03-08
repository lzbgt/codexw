use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::Cli;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::rpc::OutgoingRequest;
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
