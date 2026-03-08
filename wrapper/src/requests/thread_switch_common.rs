use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use super::PendingRequest;
use super::send_json;
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
