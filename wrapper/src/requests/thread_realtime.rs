use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use crate::requests::PendingRequest;
use crate::requests::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_thread_realtime_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    prompt: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartRealtime {
            prompt: prompt.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/realtime/start",
            params: json!({
                "threadId": thread_id,
                "prompt": prompt,
                "sessionId": state.realtime_session_id.clone(),
            }),
        },
    )
}

pub(crate) fn send_thread_realtime_append_text(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    text: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::AppendRealtimeText { text: text.clone() },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/realtime/appendText",
            params: json!({
                "threadId": thread_id,
                "text": text,
            }),
        },
    )
}

pub(crate) fn send_thread_realtime_stop(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::StopRealtime);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/realtime/stop",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}
