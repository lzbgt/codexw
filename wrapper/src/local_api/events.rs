use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use super::LocalApiSnapshot;

const MAX_STORED_EVENTS: usize = 512;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LocalApiEvent {
    pub(crate) id: u64,
    pub(crate) session_id: String,
    pub(crate) event: String,
    pub(crate) data: Value,
}

#[derive(Default)]
pub(crate) struct LocalApiEventState {
    next_id: u64,
    events: VecDeque<LocalApiEvent>,
}

pub(crate) type SharedEventLog = Arc<Mutex<LocalApiEventState>>;

pub(crate) fn new_event_log() -> SharedEventLog {
    Arc::new(Mutex::new(LocalApiEventState::default()))
}

pub(crate) fn publish_snapshot_change_events(
    log: &SharedEventLog,
    previous: Option<&LocalApiSnapshot>,
    current: &LocalApiSnapshot,
) {
    if previous.is_none_or(|snapshot| {
        session_event_payload(snapshot) != session_event_payload(current)
    }) {
        push_event(log, &current.session_id, "session.updated", session_event_payload(current));
    }

    if previous.is_none_or(|snapshot| turn_event_payload(snapshot) != turn_event_payload(current)) {
        push_event(log, &current.session_id, "turn.updated", turn_event_payload(current));
    }

    if previous.is_none_or(|snapshot| snapshot.orchestration_status != current.orchestration_status) {
        push_event(
            log,
            &current.session_id,
            "orchestration.updated",
            json!(current.orchestration_status),
        );
    }

    if previous.is_none_or(|snapshot| snapshot.workers != current.workers) {
        push_event(log, &current.session_id, "workers.updated", json!(current.workers));
    }

    if previous.is_none_or(|snapshot| snapshot.capabilities != current.capabilities) {
        push_event(
            log,
            &current.session_id,
            "capabilities.updated",
            json!(current.capabilities),
        );
    }
}

pub(crate) fn events_since(
    log: &SharedEventLog,
    session_id: &str,
    last_event_id: Option<u64>,
) -> Vec<LocalApiEvent> {
    let Ok(guard) = log.lock() else {
        return Vec::new();
    };
    guard
        .events
        .iter()
        .filter(|event| {
            event.session_id == session_id && last_event_id.is_none_or(|id| event.id > id)
        })
        .cloned()
        .collect()
}

fn push_event(log: &SharedEventLog, session_id: &str, event: &str, data: Value) {
    let Ok(mut guard) = log.lock() else {
        return;
    };
    guard.next_id += 1;
    let event_id = guard.next_id;
    guard.events.push_back(LocalApiEvent {
        id: event_id,
        session_id: session_id.to_string(),
        event: event.to_string(),
        data,
    });
    if guard.events.len() > MAX_STORED_EVENTS {
        let drop_count = guard.events.len() - MAX_STORED_EVENTS;
        guard.events.drain(..drop_count);
    }
}

fn session_event_payload(snapshot: &LocalApiSnapshot) -> Value {
    json!({
        "session_id": snapshot.session_id,
        "cwd": snapshot.cwd,
        "thread_id": snapshot.thread_id,
        "objective": snapshot.objective,
        "active_personality": snapshot.active_personality,
    })
}

fn turn_event_payload(snapshot: &LocalApiSnapshot) -> Value {
    json!({
        "session_id": snapshot.session_id,
        "active_turn_id": snapshot.active_turn_id,
        "turn_running": snapshot.turn_running,
        "started_turn_count": snapshot.started_turn_count,
        "completed_turn_count": snapshot.completed_turn_count,
    })
}
