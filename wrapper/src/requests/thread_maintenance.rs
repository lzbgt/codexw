use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_thread_compact(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::CompactThread);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/compact/start",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}

pub(crate) fn send_thread_rename(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    name: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::RenameThread { name: name.clone() },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/name/set",
            params: json!({
                "threadId": thread_id,
                "name": name,
            }),
        },
    )
}

pub(crate) fn send_clean_background_terminals(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::CleanBackgroundTerminals);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/backgroundTerminals/clean",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}
