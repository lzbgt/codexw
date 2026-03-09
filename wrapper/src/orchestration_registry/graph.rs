use std::collections::BTreeSet;

use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellJobSnapshot;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::state::AppState;

use super::LiveAgentTaskSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrchestrationDependencyEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: String,
    pub(crate) blocking: bool,
}

pub(crate) fn active_wait_task_count(state: &AppState) -> usize {
    state
        .orchestration
        .live_agent_tasks
        .values()
        .filter(|task| is_active_wait_task(task))
        .count()
}

pub(crate) fn active_sidecar_agent_task_count(state: &AppState) -> usize {
    state
        .orchestration
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
        .orchestration
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
    for task in state.orchestration.live_agent_tasks.values() {
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
    for dependency in state
        .orchestration
        .background_shells
        .capability_dependency_summaries()
    {
        let edge = OrchestrationDependencyEdge {
            from: format!("shell:{}", dependency.job_id),
            to: format!("capability:@{}", dependency.capability),
            kind: format!("dependsOnCapability:{}", dependency.status.as_str()),
            blocking: dependency.blocking
                && !matches!(
                    dependency.status,
                    BackgroundShellCapabilityDependencyState::Satisfied
                ),
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

pub(crate) fn running_shell_count_by_intent(
    state: &AppState,
    intent: BackgroundShellIntent,
) -> usize {
    state
        .orchestration
        .background_shells
        .running_count_by_intent(intent)
}

pub(crate) fn running_service_count_by_readiness(
    state: &AppState,
    readiness: BackgroundShellServiceReadiness,
) -> usize {
    state
        .orchestration
        .background_shells
        .running_service_count_by_readiness(readiness)
}

fn running_background_shells(state: &AppState) -> Vec<BackgroundShellJobSnapshot> {
    state
        .orchestration
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

fn known_agent_thread_ids(state: &AppState) -> BTreeSet<String> {
    let mut ids = state
        .orchestration
        .cached_agent_threads
        .iter()
        .map(|thread| thread.id.clone())
        .collect::<BTreeSet<_>>();
    for task in state.orchestration.live_agent_tasks.values() {
        ids.extend(task.receiver_thread_ids.iter().cloned());
    }
    ids
}

fn is_active_wait_task(task: &LiveAgentTaskSummary) -> bool {
    task.tool == "wait" && task.status == "inProgress"
}
