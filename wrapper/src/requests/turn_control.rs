use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_turn_interrupt(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    turn_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::InterruptTurn);
    let outgoing_id = request_id.clone();
    if let Err(err) = send_json(
        writer,
        &OutgoingRequest {
            id: outgoing_id,
            method: "turn/interrupt",
            params: json!({
                "threadId": thread_id,
                "turnId": turn_id,
            }),
        },
    ) {
        state.pending.remove(&request_id);
        return Err(err);
    }
    state.note_turn_interrupt_requested();
    Ok(())
}
