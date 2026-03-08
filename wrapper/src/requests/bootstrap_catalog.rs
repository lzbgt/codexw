use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::collaboration::CollaborationModeAction;
use crate::model_session::ModelsAction;
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

pub(crate) fn send_load_models(
    writer: &mut ChildStdin,
    state: &mut AppState,
    action: ModelsAction,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadModels { action });
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "model/list",
            params: json!({
                "includeHidden": false,
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

pub(crate) fn send_load_collaboration_modes(
    writer: &mut ChildStdin,
    state: &mut AppState,
    action: CollaborationModeAction,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::LoadCollaborationModes { action },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "collaborationMode/list",
            params: json!({}),
        },
    )
}

pub(crate) use crate::requests::bootstrap_search::send_fuzzy_file_search;
pub(crate) use crate::requests::bootstrap_search::send_list_threads;
