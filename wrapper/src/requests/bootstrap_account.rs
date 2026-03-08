use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

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
