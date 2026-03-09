use std::collections::BTreeMap;

use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_view::WorkerFilter;

use super::super::super::*;

#[test]
fn orchestration_guidance_prefers_blockers_then_ready_services_then_sidecars() {
    let blocked = crate::state::AppState::new(true, false);
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite shell");
    let blocked_hint = orchestration_guidance_summary(&blocked).expect("blocked guidance");
    assert!(blocked_hint.contains("blocked on 1 prerequisite shell"));
    let blocked_view = render_orchestration_guidance(&blocked);
    assert!(blocked_view.contains("Inspect :ps blockers"));
    let _ = blocked.background_shells.terminate_all_running();

    let ready = crate::state::AppState::new(true, false);
    ready
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": if cfg!(windows) {
                    "echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "printf 'READY\\n'; sleep 0.4"
                },
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready service");
    for _ in 0..40 {
        if let Some(hint) = orchestration_guidance_summary(&ready)
            && hint.contains("ready for reuse")
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let ready_hint = orchestration_guidance_summary(&ready).expect("ready guidance");
    assert!(ready_hint.contains("ready for reuse"));
    let _ = ready.background_shells.terminate_all_running();

    let mut sidecar = crate::state::AppState::new(true, false);
    sidecar.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::new(),
        },
    );
    let sidecar_hint = orchestration_guidance_summary(&sidecar).expect("sidecar guidance");
    assert!(sidecar_hint.contains("running without blocking"));
}

#[test]
fn orchestration_guidance_prioritizes_missing_blocking_service_dependencies() {
    let blocked = crate::state::AppState::new(true, false);
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent shell");
    let hint = orchestration_guidance_summary(&blocked).expect("dependency guidance");
    assert!(hint.contains("missing service capability @api.http"));
    let rendered = render_orchestration_guidance(&blocked);
    assert!(rendered.contains(":ps capabilities"));
    assert!(rendered.contains(":ps dependencies missing @api.http"));
    let blockers = render_orchestration_workers_with_filter(&blocked, WorkerFilter::Blockers);
    assert!(
        blockers.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        )
    );
    let _ = blocked.background_shells.terminate_all_running();
}

#[test]
fn guidance_filter_renders_next_action_section() {
    let mut state = crate::state::AppState::new(true, false);
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "wait".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::new(),
        },
    );
    let rendered = render_orchestration_workers_with_filter(&state, WorkerFilter::Guidance);
    assert!(rendered.contains("Next action:"));
    assert!(rendered.contains("blocked on 1 agent wait"));
    assert!(rendered.contains(":multi-agents"));
}

#[test]
fn guidance_filter_uses_concrete_wait_for_booting_blocker_provider() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start blocked prerequisite");

    let rendered = render_orchestration_guidance(&services);
    assert!(rendered.contains(":ps services booting @api.http"));
    assert!(rendered.contains(":ps wait bg-1 [timeoutMs]"));
    let _ = services.background_shells.terminate_all_running();
}
