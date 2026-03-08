use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use crate::collaboration::CollaborationModeAction;
use crate::model_session::ModelsAction;
use crate::requests::PendingRequest;
use crate::requests::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

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
