use std::collections::BTreeSet;

use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellJobSnapshot;
use crate::state::AppState;

use super::scheduler::task_role;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrchestrationDependencyEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: String,
    pub(crate) blocking: bool,
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
