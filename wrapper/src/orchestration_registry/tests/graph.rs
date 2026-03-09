use std::collections::BTreeMap;

use serde_json::json;

use crate::orchestration_view::CachedAgentThreadSummary;

use super::super::LiveAgentTaskSummary;
use super::super::graph::active_sidecar_agent_task_count;
use super::super::graph::active_wait_task_count;
use super::super::graph::blocking_dependency_count;
use super::super::graph::main_agent_state_label;
use super::super::graph::orchestration_dependency_edges;
use super::super::graph::running_shell_count_by_intent;
use super::super::graph::sidecar_dependency_count;
use super::super::graph::task_role;
use super::super::graph::wait_dependency_summary;
use super::super::tracking::track_collab_agent_task_started;

#[test]
fn wait_dependency_summary_dedupes_and_counts_receivers() {
    let mut state = crate::state::AppState::new(true, false);
    track_collab_agent_task_started(
        &mut state,
        &json!({
            "type": "collabAgentToolCall",
            "id": "wait-1",
            "tool": "wait",
            "status": "inProgress",
            "senderThreadId": "thread-main",
            "receiverThreadIds": ["thread-agent-1", "thread-agent-2"],
            "agentsStates": {}
        }),
    );
    track_collab_agent_task_started(
        &mut state,
        &json!({
            "type": "collabAgentToolCall",
            "id": "wait-2",
            "tool": "wait",
            "status": "inProgress",
            "senderThreadId": "thread-main",
            "receiverThreadIds": ["thread-agent-2"],
            "agentsStates": {}
        }),
    );

    assert_eq!(active_wait_task_count(&state), 2);
    assert_eq!(
        wait_dependency_summary(&state).as_deref(),
        Some("waiting on agents thread-agent-1, thread-agent-2")
    );
}

#[test]
fn scheduler_role_helpers_distinguish_blocked_and_sidecar_work() {
    let wait_task = LiveAgentTaskSummary {
        id: "wait-1".to_string(),
        tool: "wait".to_string(),
        status: "inProgress".to_string(),
        sender_thread_id: "thread-main".to_string(),
        receiver_thread_ids: vec!["thread-agent-1".to_string()],
        prompt: None,
        agent_statuses: BTreeMap::new(),
    };
    let spawn_task = LiveAgentTaskSummary {
        id: "spawn-1".to_string(),
        tool: "spawnAgent".to_string(),
        status: "inProgress".to_string(),
        sender_thread_id: "thread-main".to_string(),
        receiver_thread_ids: vec!["thread-agent-2".to_string()],
        prompt: None,
        agent_statuses: BTreeMap::new(),
    };

    assert_eq!(task_role(&wait_task), "blocked");
    assert_eq!(task_role(&spawn_task), "sidecar");

    let mut state = crate::state::AppState::new(true, false);
    state
        .live_agent_tasks
        .insert(wait_task.id.clone(), wait_task);
    state
        .live_agent_tasks
        .insert(spawn_task.id.clone(), spawn_task);
    assert_eq!(active_sidecar_agent_task_count(&state), 1);
    assert_eq!(main_agent_state_label(&state), "blocked");
}

#[test]
fn dependency_edges_include_wait_sidecars_and_running_background_shells() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-main".to_string());
    state.live_agent_tasks.insert(
        "wait-1".to_string(),
        LiveAgentTaskSummary {
            id: "wait-1".to_string(),
            tool: "wait".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["thread-agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::new(),
        },
    );
    state.live_agent_tasks.insert(
        "spawn-1".to_string(),
        LiveAgentTaskSummary {
            id: "spawn-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["thread-agent-2".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::new(),
        },
    );
    state.cached_agent_threads = vec![CachedAgentThreadSummary {
        id: "thread-agent-2".to_string(),
        status: "running".to_string(),
        preview: "spawned".to_string(),
        updated_at: None,
    }];
    state
        .background_shells
        .start_from_tool_with_context(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "build"
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-agent-2".to_string()),
                source_call_id: Some("call-77".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        )
        .expect("start background shell");

    let edges = orchestration_dependency_edges(&state);
    assert!(
        edges
            .iter()
            .any(|edge| edge.to == "agent:thread-agent-1" && edge.blocking)
    );
    assert!(
        edges
            .iter()
            .any(|edge| edge.to == "agent:thread-agent-2" && !edge.blocking)
    );
    assert!(edges.iter().any(|edge| edge.from == "agent:thread-agent-2"
        && edge.to == "shell:bg-1"
        && edge.kind == "backgroundShell:prerequisite"
        && edge.blocking));
    assert_eq!(blocking_dependency_count(&state), 2);
    assert_eq!(sidecar_dependency_count(&state), 1);
    assert_eq!(
        running_shell_count_by_intent(
            &state,
            crate::background_shells::BackgroundShellIntent::Prerequisite
        ),
        1
    );
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn dependency_edges_include_declared_shell_capability_dependencies() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-main".to_string());
    state
        .background_shells
        .start_from_tool_with_context(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-main".to_string()),
                source_call_id: Some("call-10".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        )
        .expect("start service");
    state
        .background_shells
        .start_from_tool_with_context(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-main".to_string()),
                source_call_id: Some("call-11".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        )
        .expect("start dependent shell");

    let edges = orchestration_dependency_edges(&state);
    assert!(edges.iter().any(|edge| {
        edge.from == "shell:bg-2"
            && edge.to == "capability:@api.http"
            && edge.kind == "dependsOnCapability:satisfied"
            && !edge.blocking
    }));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn main_agent_state_turns_blocked_for_main_prerequisite_shells() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-main".to_string());
    state
        .background_shells
        .start_from_tool_with_context(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-main".to_string()),
                source_call_id: Some("call-11".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        )
        .expect("start background shell");

    assert_eq!(main_agent_state_label(&state), "blocked");
    let _ = state.background_shells.terminate_all_running();
}
