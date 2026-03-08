use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::requests::PendingRequest;
use crate::requests::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_start_review(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    review_target: Value,
    target_description: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartReview { target_description },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "review/start",
            params: json!({
                "threadId": thread_id,
                "delivery": "inline",
                "target": review_target,
            }),
        },
    )
}
