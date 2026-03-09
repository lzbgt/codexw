use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use super::super::PendingRequest;
use super::super::send_json;
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

pub(crate) fn apply_common_session_overrides(params: &mut Value, state: &AppState) {
    if let Some(model) = state.session_overrides.model.as_ref() {
        params["model"] = model
            .as_ref()
            .map_or(Value::Null, |value| Value::String(value.clone()));
    }
    if let Some(service_tier) = state.session_overrides.service_tier.as_ref() {
        params["serviceTier"] = service_tier
            .as_ref()
            .map_or(Value::Null, |value| Value::String(value.clone()));
    }
    if let Some(personality) = state.session_overrides.personality.as_ref() {
        params["personality"] = personality
            .as_ref()
            .map_or(Value::Null, |value| Value::String(value.clone()));
    }
}
