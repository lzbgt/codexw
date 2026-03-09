use std::collections::BTreeMap;

use crate::orchestration_registry::LiveAgentTaskSummary;

use super::super::CachedAgentThreadSummary;
use super::super::orchestration_background_summary;
use super::super::orchestration_guidance_summary;
use super::super::orchestration_overview_summary;
use super::super::orchestration_prompt_suffix;
use super::super::orchestration_runtime_summary;

#[test]
fn orchestration_summary_includes_agent_status_breakdown() {
    let mut state = crate::state::AppState::new(true, false);
    state.cached_agent_threads = vec![
        CachedAgentThreadSummary {
            id: "agent-1".to_string(),
            status: "active".to_string(),
            preview: "inspect auth".to_string(),
            updated_at: Some(10),
        },
        CachedAgentThreadSummary {
            id: "agent-2".to_string(),
            status: "idle".to_string(),
            preview: "review API".to_string(),
            updated_at: Some(5),
        },
    ];
    let summary = orchestration_overview_summary(&state);
    assert!(summary.contains("main=1"));
    assert!(summary.contains("deps_blocking=0"));
    assert!(summary.contains("deps_sidecar=0"));
    assert!(summary.contains("waits=0"));
    assert!(summary.contains("sidecar_agents=0"));
    assert!(summary.contains("exec_prereqs=0"));
    assert!(summary.contains("exec_sidecars=0"));
    assert!(summary.contains("exec_services=0"));
    assert!(summary.contains("services_ready=0"));
    assert!(summary.contains("services_booting=0"));
    assert!(summary.contains("services_untracked=0"));
    assert!(summary.contains("services_conflicted=0"));
    assert!(summary.contains("service_caps=0"));
    assert!(summary.contains("service_cap_conflicts=0"));
    assert!(summary.contains("agents_live=0"));
    assert!(summary.contains("agents_cached=2"));
    assert!(summary.contains("active=1"));
    assert!(summary.contains("idle=1"));
}

#[test]
fn orchestration_runtime_summary_is_empty_when_no_workers_exist() {
    let state = crate::state::AppState::new(true, false);
    assert!(orchestration_runtime_summary(&state).is_none());
    assert!(orchestration_prompt_suffix(&state).is_none());
    assert!(orchestration_background_summary(&state).is_none());
    assert!(orchestration_guidance_summary(&state).is_none());
}

#[test]
fn orchestration_prompt_suffix_distinguishes_blockers_sidecars_services_and_terminals() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-main".to_string());
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "wait".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
        },
    );
    state
        .background_shells
        .start_from_tool(
            &serde_json::json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite shell");
    state
        .background_shells
        .start_from_tool(
            &serde_json::json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service shell");
    state.background_terminals.insert(
        "proc-1".to_string(),
        crate::background_terminals::BackgroundTerminalSummary {
            item_id: "cmd-1".to_string(),
            process_id: "proc-1".to_string(),
            command_display: "python worker.py".to_string(),
            waiting: true,
            recent_inputs: Vec::new(),
            recent_output: vec!["ready".to_string()],
        },
    );

    let suffix = orchestration_prompt_suffix(&state).expect("prompt suffix");
    assert!(suffix.contains("blocked on 1 agent wait and 1 prerequisite shell"));
    assert!(suffix.contains("1 service untracked"));
    assert!(suffix.contains("1 terminal"));
    assert!(suffix.contains(":ps to view"));
    let background = orchestration_background_summary(&state).expect("background summary");
    assert!(background.contains("prereqs=1"));
    assert!(background.contains("shell_sidecars=0"));
    assert!(background.contains("services=1"));
    assert!(background.contains("services_untracked=1"));
    assert!(background.contains("terminals=1"));
    let _ = state.background_shells.terminate_all_running();
}
