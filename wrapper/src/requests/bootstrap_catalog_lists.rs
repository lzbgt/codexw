use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use crate::requests::PendingRequest;
use crate::requests::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_load_apps(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadApps);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "app/list",
            params: json!({}),
        },
    )
}

pub(crate) fn send_load_skills(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadSkills);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "skills/list",
            params: json!({
                "cwds": [resolved_cwd],
            }),
        },
    )
}

pub(crate) fn send_load_experimental_features(
    writer: &mut ChildStdin,
    state: &mut AppState,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadExperimentalFeatures);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "experimentalFeature/list",
            params: json!({
                "limit": 200,
            }),
        },
    )
}

pub(crate) fn send_load_config(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadConfig);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "config/read",
            params: json!({}),
        },
    )
}

pub(crate) fn send_load_mcp_servers(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadMcpServers);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "mcpServerStatus/list",
            params: json!({
                "limit": 50,
            }),
        },
    )
}

pub(crate) fn send_windows_sandbox_setup_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
    mode: &str,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::WindowsSandboxSetupStart {
            mode: mode.to_string(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "windowsSandbox/setupStart",
            params: json!({
                "mode": mode,
                "cwd": resolved_cwd,
            }),
        },
    )
}
