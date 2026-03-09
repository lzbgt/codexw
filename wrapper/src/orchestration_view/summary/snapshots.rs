use std::collections::BTreeMap;

use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_terminals::server_background_terminal_count;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::blocking_dependency_count;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::orchestration_registry::sidecar_dependency_count;
use crate::state::AppState;

use crate::orchestration_view::CachedAgentThreadSummary;
use crate::orchestration_view::OrchestrationSnapshot;
use crate::orchestration_view::live_agent_tasks;

pub(crate) fn orchestration_snapshot(state: &AppState) -> OrchestrationSnapshot {
    OrchestrationSnapshot {
        main_agents: 1,
        cached_agent_threads: state.orchestration.cached_agent_threads.clone(),
        live_agent_tasks: live_agent_tasks(state),
        background_shell_jobs: state.orchestration.background_shells.job_count(),
        thread_background_terminals: server_background_terminal_count(state),
    }
}

pub(crate) fn orchestration_overview_summary(state: &AppState) -> String {
    let snapshot = orchestration_snapshot(state);
    let agent_counts = summarize_agent_status_counts(&snapshot.cached_agent_threads);
    let service_cap_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflict_count();
    let services_conflicted = state
        .orchestration
        .background_shells
        .service_conflicting_job_count();
    let cap_deps_missing = state
        .orchestration
        .background_shells
        .capability_dependency_count_by_state(BackgroundShellCapabilityDependencyState::Missing);
    let cap_deps_booting = state
        .orchestration
        .background_shells
        .capability_dependency_count_by_state(BackgroundShellCapabilityDependencyState::Booting);
    let cap_deps_ambiguous = state
        .orchestration
        .background_shells
        .capability_dependency_count_by_state(BackgroundShellCapabilityDependencyState::Ambiguous);
    let service_caps = state
        .orchestration
        .background_shells
        .unique_service_capability_count();
    format!(
        "main={} deps_blocking={} deps_sidecar={} waits={} sidecar_agents={} exec_prereqs={} exec_sidecars={} exec_services={} services_ready={} services_booting={} services_untracked={} services_conflicted={} service_caps={} service_cap_conflicts={} cap_deps_missing={} cap_deps_booting={} cap_deps_ambiguous={} agents_live={} agents_cached={}{} bg_shells={} thread_terms={}",
        snapshot.main_agents,
        blocking_dependency_count(state),
        sidecar_dependency_count(state),
        active_wait_task_count(state),
        active_sidecar_agent_task_count(state),
        running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite),
        running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
        running_shell_count_by_intent(state, BackgroundShellIntent::Service),
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready),
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting),
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked),
        services_conflicted,
        service_caps,
        service_cap_conflicts,
        cap_deps_missing,
        cap_deps_booting,
        cap_deps_ambiguous,
        snapshot.live_agent_tasks.len(),
        snapshot.cached_agent_threads.len(),
        if agent_counts.is_empty() {
            String::new()
        } else {
            format!(" {agent_counts}")
        },
        snapshot.background_shell_jobs,
        snapshot.thread_background_terminals
    )
}

pub(crate) fn orchestration_runtime_summary(state: &AppState) -> Option<String> {
    let snapshot = orchestration_snapshot(state);
    if snapshot.live_agent_tasks.is_empty()
        && snapshot.background_shell_jobs == 0
        && snapshot.thread_background_terminals == 0
        && snapshot.cached_agent_threads.is_empty()
    {
        return None;
    }
    let agent_counts = summarize_agent_status_counts(&snapshot.cached_agent_threads);
    let service_cap_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflict_count();
    let services_conflicted = state
        .orchestration
        .background_shells
        .service_conflicting_job_count();
    let cap_deps_missing = state
        .orchestration
        .background_shells
        .capability_dependency_count_by_state(BackgroundShellCapabilityDependencyState::Missing);
    let cap_deps_booting = state
        .orchestration
        .background_shells
        .capability_dependency_count_by_state(BackgroundShellCapabilityDependencyState::Booting);
    let cap_deps_ambiguous = state
        .orchestration
        .background_shells
        .capability_dependency_count_by_state(BackgroundShellCapabilityDependencyState::Ambiguous);
    let service_caps = state
        .orchestration
        .background_shells
        .unique_service_capability_count();
    Some(format!(
        "main={} deps_blocking={} deps_sidecar={} waits={} sidecar_agents={} exec_prereqs={} exec_sidecars={} exec_services={} services_ready={} services_booting={} services_untracked={} services_conflicted={} service_caps={} service_cap_conflicts={} cap_deps_missing={} cap_deps_booting={} cap_deps_ambiguous={} agent_tasks={} shells={} thread_terms={} agents={}{}",
        main_agent_state_label(state),
        blocking_dependency_count(state),
        sidecar_dependency_count(state),
        active_wait_task_count(state),
        active_sidecar_agent_task_count(state),
        running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite),
        running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
        running_shell_count_by_intent(state, BackgroundShellIntent::Service),
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready),
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting),
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked),
        services_conflicted,
        service_caps,
        service_cap_conflicts,
        cap_deps_missing,
        cap_deps_booting,
        cap_deps_ambiguous,
        snapshot.live_agent_tasks.len(),
        snapshot.background_shell_jobs,
        snapshot.thread_background_terminals,
        snapshot.cached_agent_threads.len(),
        if agent_counts.is_empty() {
            String::new()
        } else {
            format!(" {agent_counts}")
        }
    ))
}

fn summarize_agent_status_counts(agent_threads: &[CachedAgentThreadSummary]) -> String {
    if agent_threads.is_empty() {
        return String::new();
    }
    let mut counts = BTreeMap::new();
    for agent in agent_threads {
        *counts.entry(agent.status.clone()).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .map(|(status, count)| format!("{status}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}
