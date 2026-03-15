use std::sync::Arc;
use std::sync::RwLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::state::AppState;

use super::async_tools::async_tool_backpressure_snapshot;
use super::async_tools::async_tool_supervision_snapshot;
use super::async_tools::async_tool_workers_snapshot;
use super::async_tools::supervision_notice_snapshot;
use super::orchestration::orchestration_dependencies_snapshot;
use super::orchestration::orchestration_status_snapshot;
use super::types::LocalApiOrchestrationStatus;
use super::types::LocalApiSnapshot;
use super::types::LocalApiWorkersSnapshot;
use super::workers::capabilities_snapshot;
use super::workers::transcript_snapshot;
use super::workers::workers_snapshot;

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
        attachment_client_id: None,
        attachment_lease_seconds: None,
        attachment_lease_expires_at_ms: None,
        thread_id: None,
        active_turn_id: None,
        objective: None,
        turn_running: false,
        started_turn_count: 0,
        completed_turn_count: 0,
        active_personality: None,
        async_tool_supervision: None,
        async_tool_backpressure: None,
        async_tool_workers: Vec::new(),
        supervision_notice: None,
        orchestration_status: LocalApiOrchestrationStatus::default(),
        orchestration_dependencies: Vec::new(),
        workers: LocalApiWorkersSnapshot::default(),
        capabilities: Vec::new(),
        transcript: Vec::new(),
    }))
}

pub(crate) fn sync_shared_snapshot(
    snapshot: &SharedSnapshot,
    state: &AppState,
) -> LocalApiSnapshot {
    if let Ok(mut guard) = snapshot.write() {
        guard.thread_id = state.thread_id.clone();
        guard.active_turn_id = state.active_turn_id.clone();
        guard.objective = state.objective.clone();
        guard.turn_running = state.turn_running;
        guard.started_turn_count = state.started_turn_count;
        guard.completed_turn_count = state.completed_turn_count;
        guard.active_personality = state.active_personality.clone();
        guard.async_tool_supervision =
            async_tool_supervision_snapshot(&guard.session_id, &guard.cwd, state);
        guard.async_tool_backpressure =
            async_tool_backpressure_snapshot(&guard.session_id, &guard.cwd, state);
        guard.async_tool_workers = async_tool_workers_snapshot(state);
        guard.supervision_notice =
            supervision_notice_snapshot(&guard.session_id, &guard.cwd, state);
        guard.orchestration_status = orchestration_status_snapshot(state);
        guard.orchestration_dependencies = orchestration_dependencies_snapshot(state);
        guard.workers = workers_snapshot(state);
        guard.capabilities = capabilities_snapshot(state);
        guard.transcript = transcript_snapshot(state);
        return guard.clone();
    }

    LocalApiSnapshot {
        session_id: String::new(),
        cwd: String::new(),
        attachment_client_id: None,
        attachment_lease_seconds: None,
        attachment_lease_expires_at_ms: None,
        thread_id: None,
        active_turn_id: None,
        objective: None,
        turn_running: false,
        started_turn_count: 0,
        completed_turn_count: 0,
        active_personality: None,
        async_tool_supervision: None,
        async_tool_backpressure: None,
        async_tool_workers: Vec::new(),
        supervision_notice: None,
        orchestration_status: LocalApiOrchestrationStatus::default(),
        orchestration_dependencies: Vec::new(),
        workers: LocalApiWorkersSnapshot::default(),
        capabilities: Vec::new(),
        transcript: Vec::new(),
    }
}
