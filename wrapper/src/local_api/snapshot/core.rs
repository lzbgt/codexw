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
use super::types::LocalApiRuntimeInfo;
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

fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn current_hostname() -> Option<String> {
    ["HOSTNAME", "COMPUTERNAME"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn sanitize_deployment_label(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    sanitized.trim_matches('-').to_string()
}

fn suggested_deployment_id(hostname: Option<&str>) -> String {
    hostname
        .map(sanitize_deployment_label)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("codexw-{}-{}", std::env::consts::ARCH, std::process::id()))
}

fn runtime_info() -> LocalApiRuntimeInfo {
    let hostname = current_hostname();
    LocalApiRuntimeInfo {
        instance_id: format!("inst_{:x}_{:x}", std::process::id(), now_unix_millis()),
        suggested_deployment_id: suggested_deployment_id(hostname.as_deref()),
        hostname,
        process_id: std::process::id(),
        process_started_at_ms: now_unix_millis(),
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        apple_silicon: cfg!(all(target_os = "macos", target_arch = "aarch64")),
        preferred_broker_transport: "connector".to_string(),
        recommended_remote_clients: vec![
            "ios".to_string(),
            "web".to_string(),
            "terminal".to_string(),
        ],
    }
}

pub(crate) fn new_shared_snapshot(session_id: String, cwd: String) -> SharedSnapshot {
    Arc::new(RwLock::new(LocalApiSnapshot {
        runtime: runtime_info(),
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
        runtime: runtime_info(),
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
