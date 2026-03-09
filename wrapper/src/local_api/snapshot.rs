use std::sync::Arc;
use std::sync::RwLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LocalApiSnapshot {
    pub(crate) session_id: String,
    pub(crate) cwd: String,
    pub(crate) thread_id: Option<String>,
    pub(crate) active_turn_id: Option<String>,
    pub(crate) objective: Option<String>,
    pub(crate) turn_running: bool,
    pub(crate) started_turn_count: u64,
    pub(crate) completed_turn_count: u64,
    pub(crate) active_personality: Option<String>,
}

pub(crate) type SharedSnapshot = Arc<RwLock<LocalApiSnapshot>>;

pub(crate) fn new_process_session_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    format!("sess_{:x}_{:x}", std::process::id(), millis)
}

pub(crate) fn new_shared_snapshot(session_id: String, cwd: String) -> SharedSnapshot {
    Arc::new(RwLock::new(LocalApiSnapshot {
        session_id,
        cwd,
        thread_id: None,
        active_turn_id: None,
        objective: None,
        turn_running: false,
        started_turn_count: 0,
        completed_turn_count: 0,
        active_personality: None,
    }))
}

pub(crate) fn sync_shared_snapshot(snapshot: &SharedSnapshot, state: &AppState) {
    if let Ok(mut guard) = snapshot.write() {
        guard.thread_id = state.thread_id.clone();
        guard.active_turn_id = state.active_turn_id.clone();
        guard.objective = state.objective.clone();
        guard.turn_running = state.turn_running;
        guard.started_turn_count = state.started_turn_count;
        guard.completed_turn_count = state.completed_turn_count;
        guard.active_personality = state.active_personality.clone();
    }
}
