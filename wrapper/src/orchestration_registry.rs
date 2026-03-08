use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde_json::Value;

use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellJobSnapshot;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::orchestration_view::CachedAgentThreadSummary;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LiveAgentTaskSummary {
    pub(crate) id: String,
    pub(crate) tool: String,
    pub(crate) status: String,
    pub(crate) sender_thread_id: String,
    pub(crate) receiver_thread_ids: Vec<String>,
    pub(crate) prompt: Option<String>,
    pub(crate) agent_statuses: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrchestrationDependencyEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: String,
    pub(crate) blocking: bool,
}

pub(crate) fn active_wait_task_count(state: &AppState) -> usize {
    state
        .live_agent_tasks
        .values()
        .filter(|task| is_active_wait_task(task))
        .count()
}

pub(crate) fn active_sidecar_agent_task_count(state: &AppState) -> usize {
    state
        .live_agent_tasks
        .values()
        .filter(|task| task_role(task) == "sidecar")
        .count()
}

pub(crate) fn main_agent_state_label(state: &AppState) -> &'static str {
    if orchestration_dependency_edges(state)
        .iter()
        .any(|edge| edge.from == "main" && edge.blocking)
    {
        "blocked"
    } else {
        "runnable"
    }
}

pub(crate) fn blocking_dependency_count(state: &AppState) -> usize {
    orchestration_dependency_edges(state)
        .into_iter()
        .filter(|edge| edge.blocking)
        .count()
}

pub(crate) fn sidecar_dependency_count(state: &AppState) -> usize {
    orchestration_dependency_edges(state)
        .into_iter()
        .filter(|edge| !edge.blocking)
        .count()
}

pub(crate) fn wait_dependency_summary(state: &AppState) -> Option<String> {
    let waiting_on = wait_dependency_threads(state);
    match waiting_on.len() {
        0 => None,
        1 => Some(format!("waiting on agent {}", waiting_on[0])),
        _ => {
            let preview = waiting_on
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            if waiting_on.len() <= 3 {
                Some(format!("waiting on agents {preview}"))
            } else {
                Some(format!(
                    "waiting on agents {} and {} more",
                    preview,
                    waiting_on.len() - 3
                ))
            }
        }
    }
}

pub(crate) fn wait_dependency_threads(state: &AppState) -> Vec<String> {
    let mut threads = state
        .live_agent_tasks
        .values()
        .filter(|task| is_active_wait_task(task))
        .flat_map(|task| task.receiver_thread_ids.iter().cloned())
        .collect::<Vec<_>>();
    threads.sort();
    threads.dedup();
    threads
}

pub(crate) fn task_role(task: &LiveAgentTaskSummary) -> &'static str {
    if is_active_wait_task(task) {
        "blocked"
    } else if task.status == "inProgress"
        && matches!(
            task.tool.as_str(),
            "spawnAgent" | "sendInput" | "resumeAgent"
        )
    {
        "sidecar"
    } else {
        "control"
    }
}

pub(crate) fn orchestration_dependency_edges(state: &AppState) -> Vec<OrchestrationDependencyEdge> {
    let mut seen = BTreeSet::new();
    let mut edges = Vec::new();
    for task in state.live_agent_tasks.values() {
        match task_role(task) {
            "blocked" | "sidecar" => {
                for receiver in &task.receiver_thread_ids {
                    let edge = OrchestrationDependencyEdge {
                        from: "main".to_string(),
                        to: format!("agent:{receiver}"),
                        kind: task.tool.clone(),
                        blocking: task_role(task) == "blocked",
                    };
                    let dedupe = (
                        edge.from.clone(),
                        edge.to.clone(),
                        edge.kind.clone(),
                        edge.blocking,
                    );
                    if seen.insert(dedupe) {
                        edges.push(edge);
                    }
                }
            }
            _ => {}
        }
    }
    for shell in running_background_shells(state) {
        let kind = format!("backgroundShell:{}", shell.intent.as_str());
        let edge = OrchestrationDependencyEdge {
            from: shell_source_node(state, &shell),
            to: format!("shell:{}", shell.id),
            kind,
            blocking: shell.intent.is_blocking(),
        };
        let dedupe = (
            edge.from.clone(),
            edge.to.clone(),
            edge.kind.clone(),
            edge.blocking,
        );
        if seen.insert(dedupe) {
            edges.push(edge);
        }
    }
    edges.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| right.blocking.cmp(&left.blocking))
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| left.kind.cmp(&right.kind))
    });
    edges
}

pub(crate) fn track_collab_agent_task_started(state: &mut AppState, item: &Value) {
    let Some(task) = parse_live_agent_task(item) else {
        return;
    };
    merge_cached_agent_threads(state, &task, item);
    state.live_agent_tasks.insert(task.id.clone(), task);
}

pub(crate) fn track_collab_agent_task_completed(state: &mut AppState, item: &Value) {
    let Some(task) = parse_live_agent_task(item) else {
        return;
    };
    merge_cached_agent_threads(state, &task, item);
    state.live_agent_tasks.remove(&task.id);
}

fn parse_live_agent_task(item: &Value) -> Option<LiveAgentTaskSummary> {
    if get_string(item, &["type"]) != Some("collabAgentToolCall") {
        return None;
    }
    let id = get_string(item, &["id"])?;
    let tool = get_string(item, &["tool"]).unwrap_or("unknown");
    let status = get_string(item, &["status"]).unwrap_or("unknown");
    let sender_thread_id = get_string(item, &["senderThreadId"]).unwrap_or("-");
    let receiver_thread_ids = item
        .get("receiverThreadIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let prompt = get_string(item, &["prompt"])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(summarize_text);
    let agent_statuses = parse_agent_statuses(item.get("agentsStates"));
    Some(LiveAgentTaskSummary {
        id: id.to_string(),
        tool: tool.to_string(),
        status: status.to_string(),
        sender_thread_id: sender_thread_id.to_string(),
        receiver_thread_ids,
        prompt,
        agent_statuses,
    })
}

fn running_background_shells(state: &AppState) -> Vec<BackgroundShellJobSnapshot> {
    state
        .background_shells
        .snapshots()
        .into_iter()
        .filter(|job| job.status == "running")
        .collect()
}

fn shell_source_node(state: &AppState, shell: &BackgroundShellJobSnapshot) -> String {
    let Some(source_thread_id) = shell.origin.source_thread_id.as_deref() else {
        return "main".to_string();
    };
    if state.thread_id.as_deref() == Some(source_thread_id) {
        return "main".to_string();
    }
    if known_agent_thread_ids(state).contains(source_thread_id) {
        return format!("agent:{source_thread_id}");
    }
    format!("thread:{source_thread_id}")
}

pub(crate) fn running_shell_count_by_intent(
    state: &AppState,
    intent: BackgroundShellIntent,
) -> usize {
    state.background_shells.running_count_by_intent(intent)
}

pub(crate) fn running_service_count_by_readiness(
    state: &AppState,
    readiness: BackgroundShellServiceReadiness,
) -> usize {
    state
        .background_shells
        .running_service_count_by_readiness(readiness)
}

fn known_agent_thread_ids(state: &AppState) -> BTreeSet<String> {
    let mut ids = state
        .cached_agent_threads
        .iter()
        .map(|thread| thread.id.clone())
        .collect::<BTreeSet<_>>();
    for task in state.live_agent_tasks.values() {
        ids.extend(task.receiver_thread_ids.iter().cloned());
    }
    ids
}

fn is_active_wait_task(task: &LiveAgentTaskSummary) -> bool {
    task.tool == "wait" && task.status == "inProgress"
}

fn parse_agent_statuses(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|states| {
            states
                .iter()
                .map(|(thread_id, state)| {
                    let status = get_string(state, &["status"]).unwrap_or("unknown");
                    (thread_id.clone(), status.to_string())
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn merge_cached_agent_threads(state: &mut AppState, task: &LiveAgentTaskSummary, item: &Value) {
    for thread_id in &task.receiver_thread_ids {
        let status = task
            .agent_statuses
            .get(thread_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let preview = item
            .get("agentsStates")
            .and_then(|states| states.get(thread_id))
            .and_then(|agent| get_string(agent, &["message"]))
            .map(summarize_text)
            .or_else(|| task.prompt.clone())
            .unwrap_or_else(|| "-".to_string());
        upsert_cached_agent_thread(
            &mut state.cached_agent_threads,
            CachedAgentThreadSummary {
                id: thread_id.clone(),
                status,
                preview,
                updated_at: None,
            },
        );
    }
}

fn upsert_cached_agent_thread(
    cached: &mut Vec<CachedAgentThreadSummary>,
    incoming: CachedAgentThreadSummary,
) {
    if let Some(existing) = cached.iter_mut().find(|thread| thread.id == incoming.id) {
        existing.status = incoming.status;
        if incoming.preview != "-" {
            existing.preview = incoming.preview;
        }
        if incoming.updated_at.is_some() {
            existing.updated_at = incoming.updated_at;
        }
        return;
    }
    cached.push(incoming);
    cached.sort_by(|left, right| left.id.cmp(&right.id));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::orchestration_registry::LiveAgentTaskSummary;
    use crate::orchestration_view::CachedAgentThreadSummary;

    use super::active_sidecar_agent_task_count;
    use super::active_wait_task_count;
    use super::blocking_dependency_count;
    use super::main_agent_state_label;
    use super::orchestration_dependency_edges;
    use super::running_shell_count_by_intent;
    use super::sidecar_dependency_count;
    use super::task_role;
    use super::track_collab_agent_task_completed;
    use super::track_collab_agent_task_started;
    use super::wait_dependency_summary;
    use serde_json::json;

    #[test]
    fn collab_agent_task_tracking_updates_live_registry_and_cached_threads() {
        let mut state = crate::state::AppState::new(true, false);
        let item = json!({
            "type": "collabAgentToolCall",
            "id": "call-1",
            "tool": "spawnAgent",
            "status": "inProgress",
            "senderThreadId": "thread-main",
            "receiverThreadIds": ["thread-agent-1"],
            "prompt": "Inspect auth flow and report risks",
            "agentsStates": {
                "thread-agent-1": {
                    "status": "running",
                    "message": "reviewing auth flow"
                }
            }
        });

        track_collab_agent_task_started(&mut state, &item);

        assert_eq!(state.live_agent_tasks.len(), 1);
        assert_eq!(state.cached_agent_threads.len(), 1);
        assert_eq!(state.cached_agent_threads[0].id, "thread-agent-1");
        assert_eq!(state.cached_agent_threads[0].status, "running");
        assert_eq!(state.cached_agent_threads[0].preview, "reviewing auth flow");

        let completed = json!({
            "type": "collabAgentToolCall",
            "id": "call-1",
            "tool": "spawnAgent",
            "status": "completed",
            "senderThreadId": "thread-main",
            "receiverThreadIds": ["thread-agent-1"],
            "agentsStates": {
                "thread-agent-1": {
                    "status": "completed",
                    "message": "done"
                }
            }
        });
        track_collab_agent_task_completed(&mut state, &completed);

        assert!(state.live_agent_tasks.is_empty());
        assert_eq!(state.cached_agent_threads[0].status, "completed");
        assert_eq!(state.cached_agent_threads[0].preview, "done");
    }

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
}
