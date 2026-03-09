use std::collections::BTreeMap;

use crate::orchestration_registry::LiveAgentTaskSummary;

use super::CachedAgentThreadSummary;
use super::WorkerFilter;
use super::orchestration_background_summary;
use super::orchestration_guidance_summary;
use super::orchestration_overview_summary;
use super::orchestration_prompt_suffix;
use super::orchestration_runtime_summary;
use super::render_orchestration_actions;
use super::render_orchestration_actions_for_capability;
use super::render_orchestration_actions_for_tool;
use super::render_orchestration_actions_for_tool_capability;
use super::render_orchestration_blockers_for_capability;
use super::render_orchestration_guidance;
use super::render_orchestration_guidance_for_capability;
use super::render_orchestration_workers;
use super::render_orchestration_workers_with_filter;

#[path = "tests/guidance_actions.rs"]
mod guidance_actions;

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

#[test]
fn orchestration_worker_rendering_includes_cached_agents_and_background_tasks() {
    let mut state = crate::state::AppState::new(true, false);
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: Some("inspect auth".to_string()),
            agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
        },
    );
    state.live_agent_tasks.insert(
        "call-2".to_string(),
        LiveAgentTaskSummary {
            id: "call-2".to_string(),
            tool: "wait".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
        },
    );
    state.cached_agent_threads = vec![CachedAgentThreadSummary {
        id: "agent-1".to_string(),
        status: "active".to_string(),
        preview: "inspect auth".to_string(),
        updated_at: Some(10),
    }];
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

    let rendered = render_orchestration_workers(&state);
    assert!(rendered.contains("Main agent state: blocked | waiting on agent agent-1"));
    assert!(rendered.contains(
        "sidecar agents=1 | exec prereqs=0 | exec sidecars=0 | exec services=0 (ready=0 booting=0 untracked=0 conflicted=0) | deps blocking=1 sidecar=1"
    ));
    assert!(rendered.contains("Dependencies:"));
    assert!(rendered.contains("main -> agent:agent-1  [wait, blocking]"));
    assert!(rendered.contains("main -> agent:agent-1  [spawnAgent]"));
    assert!(rendered.contains("Live agent tasks:"));
    assert!(rendered.contains("spawnAgent  [inProgress]  thread-main -> agent-1"));
    assert!(rendered.contains("wait  [inProgress]  thread-main -> agent-1"));
    assert!(rendered.contains("role     sidecar"));
    assert!(rendered.contains("role     blocked"));
    assert!(rendered.contains("blocking yes"));
    assert!(rendered.contains("Cached agent threads:"));
    assert!(rendered.contains("agent-1  [active]"));
    assert!(rendered.contains("inspect auth"));
    assert!(rendered.contains("Use :multi-agents to refresh or switch agent threads."));
    assert!(rendered.contains("Server-observed background terminals:"));
    assert!(rendered.contains("python worker.py"));
}

#[test]
fn filtered_worker_rendering_can_target_blockers_services_and_terminals() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-main".to_string());
    state.live_agent_tasks.insert(
        "call-wait".to_string(),
        LiveAgentTaskSummary {
            id: "call-wait".to_string(),
            tool: "wait".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
        },
    );
    state.cached_agent_threads = vec![CachedAgentThreadSummary {
        id: "agent-1".to_string(),
        status: "active".to_string(),
        preview: "inspect auth".to_string(),
        updated_at: Some(10),
    }];
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
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev server",
                "capabilities": ["frontend.dev"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "Open the dev server in a browser",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health"
                    }
                ]
            }),
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

    let blockers = render_orchestration_workers_with_filter(&state, WorkerFilter::Blockers);
    assert!(blockers.contains("Dependencies:"));
    assert!(blockers.contains("wait, blocking"));
    assert!(blockers.contains("backgroundShell:prerequisite, blocking"));
    assert!(!blockers.contains("Cached agent threads:"));
    assert!(!blockers.contains("Server-observed background terminals:"));

    let dependencies = render_orchestration_workers_with_filter(&state, WorkerFilter::Dependencies);
    assert!(dependencies.contains("Dependencies:"));
    assert!(!dependencies.contains("Main agent state:"));
    assert!(dependencies.contains("main -> agent:agent-1  [wait, blocking]"));
    assert!(dependencies.contains("main -> shell:bg-1  [backgroundShell:prerequisite, blocking]"));

    let services = render_orchestration_workers_with_filter(&state, WorkerFilter::Services);
    assert!(services.contains("Local background shell jobs:"));
    assert!(services.contains("intent   service"));
    assert!(services.contains("label    dev server"));
    assert!(services.contains("protocol http"));
    assert!(services.contains("endpoint http://127.0.0.1:3000"));
    assert!(services.contains("attach   Open the dev server in a browser"));
    assert!(services.contains("recipes  1"));
    assert!(services.contains("service  untracked"));
    assert!(services.contains("Capability index:"));
    assert!(!services.contains("intent   prerequisite"));

    let capabilities = render_orchestration_workers_with_filter(&state, WorkerFilter::Capabilities);
    assert!(capabilities.contains("Service capability index:"));
    assert!(!capabilities.contains("Local background shell jobs:"));

    let terminals = render_orchestration_workers_with_filter(&state, WorkerFilter::Terminals);
    assert!(terminals.contains("Server-observed background terminals:"));
    assert!(terminals.contains("python worker.py"));
    assert!(!terminals.contains("Local background shell jobs:"));
    let _ = state.background_shells.terminate_all_running();
}
