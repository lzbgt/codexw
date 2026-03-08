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

pub(crate) fn send_thread_switch_request(
    writer: &mut ChildStdin,
    state: &mut AppState,
    method: &'static str,
    pending: PendingRequest,
    params: Value,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending_thread_switch = true;
    state.pending.insert(request_id.clone(), pending);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method,
            params,
        },
    )
}

pub(crate) fn send_thread_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    initial_prompt: Option<String>,
) -> Result<()> {
    send_thread_switch_request(
        writer,
        state,
        "thread/start",
        PendingRequest::StartThread { initial_prompt },
        json!({
            "model": cli.model,
            "modelProvider": cli.model_provider,
            "cwd": resolved_cwd,
            "approvalPolicy": approval_policy(cli),
            "sandbox": thread_sandbox_mode(cli),
            "serviceName": "codexw_terminal",
            "experimentalRawEvents": false,
        }),
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
    send_thread_switch_request(
        writer,
        state,
        "thread/resume",
        PendingRequest::ResumeThread { initial_prompt },
        json!({
            "threadId": thread_id,
            "model": cli.model,
            "modelProvider": cli.model_provider,
            "cwd": resolved_cwd,
            "approvalPolicy": approval_policy(cli),
            "sandbox": thread_sandbox_mode(cli),
        }),
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
    send_thread_switch_request(
        writer,
        state,
        "thread/fork",
        PendingRequest::ForkThread { initial_prompt },
        json!({
            "threadId": thread_id,
            "cwd": resolved_cwd,
            "model": cli.model,
            "modelProvider": cli.model_provider,
            "approvalPolicy": approval_policy(cli),
            "sandbox": thread_sandbox_mode(cli),
        }),
    )
}
