use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::state::AppState;

use super::super::LiveAgentTaskSummary;
use super::edges::orchestration_dependency_edges;

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

fn wait_dependency_threads(state: &AppState) -> Vec<String> {
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

fn is_active_wait_task(task: &LiveAgentTaskSummary) -> bool {
    task.tool == "wait" && task.status == "inProgress"
}
