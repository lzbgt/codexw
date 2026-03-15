use serde_json::json;

use crate::local_api::LocalApiSnapshot;

use super::attachment::now_unix_ms;

pub(super) fn session_payload(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    let session = session_summary(snapshot);
    json!({
        "ok": true,
        "session": session,
        "session_id": snapshot.session_id,
        "cwd": snapshot.cwd,
        "thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
        "objective": snapshot.objective,
        "working": snapshot.turn_running,
        "started_turn_count": snapshot.started_turn_count,
        "completed_turn_count": snapshot.completed_turn_count,
        "active_personality": snapshot.active_personality,
        "async_tool_supervision": snapshot.async_tool_supervision,
        "async_tool_backpressure": snapshot.async_tool_backpressure,
        "async_tool_workers": snapshot.async_tool_workers,
        "supervision_notice": snapshot.supervision_notice,
        "orchestration": snapshot.orchestration_status,
        "transcript_length": snapshot.transcript.len(),
    })
}

pub(super) fn attachment_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    let lease_active = snapshot
        .attachment_lease_expires_at_ms
        .is_some_and(|expiry| expiry > now_unix_ms());
    json!({
        "id": format!("attach:{}", snapshot.session_id),
        "scope": "process",
        "process_scoped": true,
        "client_id": snapshot.attachment_client_id,
        "lease_seconds": snapshot.attachment_lease_seconds,
        "lease_expires_at_ms": snapshot.attachment_lease_expires_at_ms,
        "lease_active": lease_active,
        "attached_thread_id": snapshot.thread_id,
    })
}

pub(super) fn session_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    json!({
        "id": snapshot.session_id,
        "scope": "process",
        "process_scoped": true,
        "attachment": attachment_summary(snapshot),
        "client_id": snapshot.attachment_client_id,
        "cwd": snapshot.cwd,
        "attached_thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
        "objective": snapshot.objective,
        "working": snapshot.turn_running,
        "started_turn_count": snapshot.started_turn_count,
        "completed_turn_count": snapshot.completed_turn_count,
        "active_personality": snapshot.active_personality,
        "async_tool_supervision": snapshot.async_tool_supervision,
        "async_tool_backpressure": snapshot.async_tool_backpressure,
        "async_tool_workers": snapshot.async_tool_workers,
        "supervision_notice": snapshot.supervision_notice,
        "transcript_length": snapshot.transcript.len(),
    })
}
