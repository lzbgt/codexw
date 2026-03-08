use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::Cli;
use crate::rpc::OutgoingNotification;
use crate::rpc::OutgoingRequest;
use crate::session::CollaborationModeAction;
use crate::session::ModelsAction;
use crate::state::AppState;

pub(crate) fn send_initialize(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    experimental_api: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::Initialize);
    let mut capabilities = json!({
        "experimentalApi": experimental_api,
    });
    if !cli.raw_json {
        capabilities["optOutNotificationMethods"] = json!([
            "item/agentMessage/delta",
            "item/reasoning/summaryTextDelta",
            "item/reasoning/summaryPartAdded",
            "item/reasoning/textDelta",
            "item/plan/delta"
        ]);
    }
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "initialize",
            params: json!({
                "clientInfo": {
                    "name": "codexw_terminal",
                    "title": "CodexW Terminal",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": capabilities
            }),
        },
    )
}

pub(crate) fn send_initialized(writer: &mut ChildStdin) -> Result<()> {
    send_json(
        writer,
        &OutgoingNotification {
            method: "initialized",
            params: Value::Null,
        },
    )
}

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

pub(crate) fn send_load_account(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadAccount);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "account/read",
            params: json!({
                "refreshToken": false,
            }),
        },
    )
}

pub(crate) fn send_logout_account(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LogoutAccount);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "account/logout",
            params: json!({}),
        },
    )
}

pub(crate) fn send_feedback_upload(
    writer: &mut ChildStdin,
    state: &mut AppState,
    classification: String,
    reason: Option<String>,
    thread_id: Option<String>,
    include_logs: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::UploadFeedback {
            classification: classification.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "feedback/upload",
            params: json!({
                "classification": classification,
                "reason": reason,
                "threadId": thread_id,
                "includeLogs": include_logs,
            }),
        },
    )
}

pub(crate) fn send_load_rate_limits(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadRateLimits);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "account/rateLimits/read",
            params: json!({}),
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

pub(crate) fn send_list_threads(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
    search_term: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ListThreads {
            search_term: search_term.clone(),
        },
    );
    let mut params = json!({
        "limit": 10,
        "sortKey": "updated_at",
        "cwd": resolved_cwd,
    });
    if let Some(search_term) = search_term {
        params["searchTerm"] = Value::String(search_term);
    }
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/list",
            params,
        },
    )
}

pub(crate) fn send_fuzzy_file_search(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
    query: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::FuzzyFileSearch {
            query: query.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "fuzzyFileSearch",
            params: json!({
                "query": query,
                "roots": [resolved_cwd],
            }),
        },
    )
}
