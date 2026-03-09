use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_terminals::server_background_terminal_count;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::state::AppState;

use crate::orchestration_view::pluralize;

pub(crate) fn orchestration_prompt_suffix(state: &AppState) -> Option<String> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecars = active_sidecar_agent_task_count(state)
        + running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let services_ready =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let services_booting =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    let services_untracked =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    let services_conflicted = state
        .orchestration
        .background_shells
        .service_conflicting_job_count();
    let service_caps = state
        .orchestration
        .background_shells
        .unique_service_capability_count();
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
    let terminals = server_background_terminal_count(state);
    if waits == 0
        && prereqs == 0
        && sidecars == 0
        && services_ready == 0
        && services_booting == 0
        && services_untracked == 0
        && services_conflicted == 0
        && cap_deps_missing == 0
        && cap_deps_booting == 0
        && cap_deps_ambiguous == 0
        && terminals == 0
    {
        return None;
    }

    let mut parts = Vec::new();
    if main_agent_state_label(state) == "blocked" {
        if waits > 0 && prereqs > 0 {
            parts.push(format!(
                "blocked on {} and {}",
                pluralize(waits, "agent wait", "agent waits"),
                pluralize(prereqs, "prerequisite shell", "prerequisite shells")
            ));
        } else if waits > 0 {
            parts.push(format!(
                "blocked on {}",
                pluralize(waits, "agent wait", "agent waits")
            ));
        } else if prereqs > 0 {
            parts.push(format!(
                "blocked on {}",
                pluralize(prereqs, "prerequisite shell", "prerequisite shells")
            ));
        }
    }
    if sidecars > 0 {
        parts.push(pluralize(sidecars, "sidecar", "sidecars"));
    }
    if services_booting > 0 {
        parts.push(format!(
            "{} booting",
            pluralize(services_booting, "service", "services")
        ));
    }
    if services_ready > 0 {
        parts.push(format!(
            "{} ready",
            pluralize(services_ready, "service", "services")
        ));
    }
    if services_untracked > 0 {
        parts.push(format!(
            "{} untracked",
            pluralize(services_untracked, "service", "services")
        ));
    }
    if services_conflicted > 0 {
        parts.push(format!(
            "{} conflicted",
            pluralize(services_conflicted, "service", "services")
        ));
    }
    if service_caps > 0 {
        parts.push(pluralize(
            service_caps,
            "service capability",
            "service capabilities",
        ));
    }
    if cap_deps_missing > 0 {
        parts.push(format!("{} missing deps", cap_deps_missing));
    }
    if cap_deps_booting > 0 {
        parts.push(format!("{} booting deps", cap_deps_booting));
    }
    if cap_deps_ambiguous > 0 {
        parts.push(format!("{} ambiguous deps", cap_deps_ambiguous));
    }
    if terminals > 0 {
        parts.push(pluralize(terminals, "terminal", "terminals"));
    }
    parts.push(":ps to view".to_string());
    parts.push(":clean to close".to_string());
    Some(parts.join(" | "))
}

pub(crate) fn orchestration_background_summary(state: &AppState) -> Option<String> {
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let services = running_shell_count_by_intent(state, BackgroundShellIntent::Service);
    let ready = running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let booting =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    let untracked =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    let conflicted = state
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
    let terminals = server_background_terminal_count(state);
    if prereqs == 0
        && sidecars == 0
        && services == 0
        && conflicted == 0
        && cap_deps_missing == 0
        && cap_deps_booting == 0
        && cap_deps_ambiguous == 0
        && terminals == 0
    {
        None
    } else {
        Some(format!(
            "prereqs={prereqs} shell_sidecars={sidecars} services={services} services_ready={ready} services_booting={booting} services_untracked={untracked} services_conflicted={conflicted} cap_deps_missing={cap_deps_missing} cap_deps_booting={cap_deps_booting} cap_deps_ambiguous={cap_deps_ambiguous} terminals={terminals}"
        ))
    }
}
