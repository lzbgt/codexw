use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::blocking_dependency_count;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::orchestration_dependency_edges;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::orchestration_registry::sidecar_dependency_count;
use crate::orchestration_registry::wait_dependency_summary;
use crate::state::AppState;

use super::LocalApiDependencyEdge;
use super::LocalApiOrchestrationStatus;

pub(super) fn orchestration_status_snapshot(state: &AppState) -> LocalApiOrchestrationStatus {
    LocalApiOrchestrationStatus {
        main_agent_state: main_agent_state_label(state).to_string(),
        wait_summary: wait_dependency_summary(state),
        blocking_dependencies: blocking_dependency_count(state),
        sidecar_dependencies: sidecar_dependency_count(state),
        wait_tasks: active_wait_task_count(state),
        sidecar_agent_tasks: active_sidecar_agent_task_count(state),
        exec_prerequisites: running_shell_count_by_intent(
            state,
            BackgroundShellIntent::Prerequisite,
        ),
        exec_sidecars: running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
        exec_services: running_shell_count_by_intent(state, BackgroundShellIntent::Service),
        services_ready: running_service_count_by_readiness(
            state,
            BackgroundShellServiceReadiness::Ready,
        ),
        services_booting: running_service_count_by_readiness(
            state,
            BackgroundShellServiceReadiness::Booting,
        ),
        services_untracked: running_service_count_by_readiness(
            state,
            BackgroundShellServiceReadiness::Untracked,
        ),
        services_conflicted: state
            .orchestration
            .background_shells
            .service_conflicting_job_count(),
        service_capabilities: state
            .orchestration
            .background_shells
            .unique_service_capability_count(),
        service_capability_conflicts: state
            .orchestration
            .background_shells
            .service_capability_conflict_count(),
        capability_dependencies_missing: state
            .orchestration
            .background_shells
            .capability_dependency_count_by_state(
                crate::background_shells::BackgroundShellCapabilityDependencyState::Missing,
            ),
        capability_dependencies_booting: state
            .orchestration
            .background_shells
            .capability_dependency_count_by_state(
                crate::background_shells::BackgroundShellCapabilityDependencyState::Booting,
            ),
        capability_dependencies_ambiguous: state
            .orchestration
            .background_shells
            .capability_dependency_count_by_state(
                crate::background_shells::BackgroundShellCapabilityDependencyState::Ambiguous,
            ),
        live_agent_task_count: state.orchestration.live_agent_tasks.len(),
        cached_agent_thread_count: state.orchestration.cached_agent_threads.len(),
        background_shell_job_count: state.orchestration.background_shells.job_count(),
        background_terminal_count: state.orchestration.background_terminals.len(),
    }
}

pub(super) fn orchestration_dependencies_snapshot(state: &AppState) -> Vec<LocalApiDependencyEdge> {
    orchestration_dependency_edges(state)
        .into_iter()
        .map(|edge| LocalApiDependencyEdge {
            from: edge.from,
            to: edge.to,
            kind: edge.kind,
            blocking: edge.blocking,
        })
        .collect()
}
