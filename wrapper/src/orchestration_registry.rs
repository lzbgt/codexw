#[path = "orchestration_registry/graph.rs"]
mod graph;
#[path = "orchestration_registry/tracking.rs"]
mod tracking;

use std::collections::BTreeMap;

pub(crate) use self::graph::OrchestrationDependencyEdge;
pub(crate) use self::graph::active_sidecar_agent_task_count;
pub(crate) use self::graph::active_wait_task_count;
pub(crate) use self::graph::blocking_dependency_count;
pub(crate) use self::graph::main_agent_state_label;
pub(crate) use self::graph::orchestration_dependency_edges;
pub(crate) use self::graph::running_service_count_by_readiness;
pub(crate) use self::graph::running_shell_count_by_intent;
pub(crate) use self::graph::sidecar_dependency_count;
pub(crate) use self::graph::task_role;
pub(crate) use self::graph::wait_dependency_summary;
pub(crate) use self::graph::wait_dependency_threads;
pub(crate) use self::tracking::track_collab_agent_task_completed;
pub(crate) use self::tracking::track_collab_agent_task_started;

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

#[cfg(test)]
#[path = "orchestration_registry/tests.rs"]
mod tests;
