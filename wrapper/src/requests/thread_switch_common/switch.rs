use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use super::shared::apply_common_session_overrides;
use super::shared::send_thread_switch_request;
use crate::Cli;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::requests::PendingRequest;
use crate::state::AppState;

pub(crate) fn send_thread_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    initial_prompt: Option<String>,
) -> Result<()> {
    let mut params = json!({
        "model": cli.model,
        "modelProvider": cli.model_provider,
        "cwd": resolved_cwd,
        "approvalPolicy": approval_policy(cli, state),
        "sandbox": thread_sandbox_mode(cli, state),
        "serviceName": "codexw_terminal",
        "experimentalRawEvents": false,
        "persistExtendedHistory": true,
    });
    apply_common_session_overrides(&mut params, state);
    send_thread_switch_request(
        writer,
        state,
        "thread/start",
        PendingRequest::StartThread { initial_prompt },
        params,
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
    let mut params = json!({
        "threadId": thread_id,
        "model": cli.model,
        "modelProvider": cli.model_provider,
        "cwd": resolved_cwd,
        "approvalPolicy": approval_policy(cli, state),
        "sandbox": thread_sandbox_mode(cli, state),
        "persistExtendedHistory": true,
    });
    apply_common_session_overrides(&mut params, state);
    send_thread_switch_request(
        writer,
        state,
        "thread/resume",
        PendingRequest::ResumeThread { initial_prompt },
        params,
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
    let mut params = json!({
        "threadId": thread_id,
        "cwd": resolved_cwd,
        "model": cli.model,
        "modelProvider": cli.model_provider,
        "approvalPolicy": approval_policy(cli, state),
        "sandbox": thread_sandbox_mode(cli, state),
        "persistExtendedHistory": true,
    });
    apply_common_session_overrides(&mut params, state);
    send_thread_switch_request(
        writer,
        state,
        "thread/fork",
        PendingRequest::ForkThread { initial_prompt },
        params,
    )
}
