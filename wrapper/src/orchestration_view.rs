use std::collections::BTreeMap;

use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_terminals::render_background_terminals;
use crate::background_terminals::server_background_terminal_count;
use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::blocking_dependency_count;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::orchestration_dependency_edges;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::orchestration_registry::sidecar_dependency_count;
use crate::orchestration_registry::task_role;
use crate::orchestration_registry::wait_dependency_summary;
use crate::state::AppState;
use crate::state::summarize_text;

mod guidance_actions;

#[cfg(test)]
pub(crate) use guidance_actions::orchestration_guidance_summary;
pub(crate) use guidance_actions::orchestration_next_action_summary;
pub(crate) use guidance_actions::orchestration_next_action_summary_for_tool;
pub(crate) use guidance_actions::render_orchestration_actions;
pub(crate) use guidance_actions::render_orchestration_actions_for_capability;
pub(crate) use guidance_actions::render_orchestration_actions_for_tool;
pub(crate) use guidance_actions::render_orchestration_actions_for_tool_capability;
pub(crate) use guidance_actions::render_orchestration_blockers_for_capability;
pub(crate) use guidance_actions::render_orchestration_guidance;
pub(crate) use guidance_actions::render_orchestration_guidance_for_capability;
pub(crate) use guidance_actions::render_orchestration_guidance_for_tool;
pub(crate) use guidance_actions::render_orchestration_guidance_for_tool_capability;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerFilter {
    All,
    Blockers,
    Dependencies,
    Agents,
    Shells,
    Services,
    Capabilities,
    Terminals,
    Guidance,
    Actions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DependencyFilter {
    All,
    Blocking,
    Sidecars,
    Missing,
    Booting,
    Ambiguous,
    Satisfied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DependencySelection {
    pub(crate) filter: DependencyFilter,
    pub(crate) capability: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CachedAgentThreadSummary {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) preview: String,
    pub(crate) updated_at: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OrchestrationSnapshot {
    pub(crate) main_agents: usize,
    pub(crate) cached_agent_threads: Vec<CachedAgentThreadSummary>,
    pub(crate) live_agent_tasks: Vec<LiveAgentTaskSummary>,
    pub(crate) background_shell_jobs: usize,
    pub(crate) thread_background_terminals: usize,
}

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
        parts.push(format!(
            "{}",
            pluralize(service_caps, "service capability", "service capabilities")
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

pub(crate) fn render_orchestration_workers(state: &AppState) -> String {
    render_orchestration_workers_with_filter(state, WorkerFilter::All)
}

pub(crate) fn render_orchestration_workers_with_filter(
    state: &AppState,
    filter: WorkerFilter,
) -> String {
    if matches!(filter, WorkerFilter::Guidance) {
        let guidance = render_orchestration_guidance(state);
        if guidance.is_empty() {
            return empty_filter_message(filter).to_string();
        }
        return guidance;
    }
    if matches!(filter, WorkerFilter::Actions) {
        let actions = render_orchestration_actions(state);
        if actions.is_empty() {
            return empty_filter_message(filter).to_string();
        }
        return actions;
    }
    let mut lines = Vec::new();
    if matches!(
        filter,
        WorkerFilter::All | WorkerFilter::Blockers | WorkerFilter::Dependencies
    ) {
        lines.extend(render_main_agent_section(state, filter));
    }
    if matches!(
        filter,
        WorkerFilter::All | WorkerFilter::Agents | WorkerFilter::Blockers
    ) {
        let tasks = live_agent_tasks(state)
            .into_iter()
            .filter(|task| match filter {
                WorkerFilter::All | WorkerFilter::Agents => true,
                WorkerFilter::Blockers => task.tool == "wait" && task.status == "inProgress",
                _ => false,
            })
            .collect::<Vec<_>>();
        if !tasks.is_empty() {
            push_section_gap(&mut lines);
            lines.extend(render_live_agent_tasks_section(&tasks));
        }
    }
    if matches!(filter, WorkerFilter::All | WorkerFilter::Agents)
        && !state.orchestration.cached_agent_threads.is_empty()
    {
        push_section_gap(&mut lines);
        lines.extend(render_cached_agent_threads_section(
            &state.orchestration.cached_agent_threads,
        ));
    }
    let shell_lines = match filter {
        WorkerFilter::All | WorkerFilter::Shells => {
            state.orchestration.background_shells.render_for_ps()
        }
        WorkerFilter::Services => state
            .orchestration
            .background_shells
            .render_service_shells_for_ps_filtered(None, None),
        WorkerFilter::Capabilities => state
            .orchestration
            .background_shells
            .render_service_capabilities_for_ps(),
        WorkerFilter::Blockers => state
            .orchestration
            .background_shells
            .render_for_ps_filtered(Some(BackgroundShellIntent::Prerequisite)),
        _ => None,
    };
    if let Some(shell_lines) = shell_lines {
        push_section_gap(&mut lines);
        lines.extend(shell_lines);
    }
    if matches!(filter, WorkerFilter::All | WorkerFilter::Terminals)
        && let Some(terminal_lines) = render_server_background_terminals_only(state)
    {
        push_section_gap(&mut lines);
        lines.extend(terminal_lines);
    }
    if lines.is_empty() {
        return empty_filter_message(filter).to_string();
    }
    lines.join("\n")
}

pub(crate) fn render_orchestration_dependencies(
    state: &AppState,
    selection: &DependencySelection,
) -> String {
    let lines = render_dependency_section(state, selection);
    if lines.is_empty() {
        return empty_dependency_filter_message(selection).to_string();
    }
    lines.join("\n")
}

fn live_agent_tasks(state: &AppState) -> Vec<LiveAgentTaskSummary> {
    let mut tasks = state
        .orchestration
        .live_agent_tasks
        .values()
        .cloned()
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    tasks
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

fn render_main_agent_section(state: &AppState, filter: WorkerFilter) -> Vec<String> {
    let mut lines = Vec::new();
    if !matches!(filter, WorkerFilter::Dependencies) {
        let mut main_line = format!("Main agent state: {}", main_agent_state_label(state));
        if let Some(waiting_on) = wait_dependency_summary(state) {
            main_line.push_str(&format!(" | {waiting_on}"));
        }
        main_line.push_str(&format!(
            " | sidecar agents={} | exec prereqs={} | exec sidecars={} | exec services={} (ready={} booting={} untracked={} conflicted={}) | deps blocking={} sidecar={}",
            active_sidecar_agent_task_count(state),
            running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite),
            running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
            running_shell_count_by_intent(state, BackgroundShellIntent::Service),
            running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready),
            running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting),
            running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked),
            state
                .orchestration
                .background_shells
                .service_conflicting_job_count(),
            blocking_dependency_count(state),
            sidecar_dependency_count(state)
        ));
        lines.push(main_line);
    }
    let dependency_selection = if matches!(filter, WorkerFilter::Blockers) {
        DependencySelection {
            filter: DependencyFilter::Blocking,
            capability: None,
        }
    } else {
        DependencySelection {
            filter: DependencyFilter::All,
            capability: None,
        }
    };
    let dependency_lines = render_dependency_section(state, &dependency_selection);
    if !dependency_lines.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(dependency_lines);
    }
    lines
}

fn render_dependency_section(state: &AppState, selection: &DependencySelection) -> Vec<String> {
    let dependencies = orchestration_dependency_edges(state)
        .into_iter()
        .filter(|edge| dependency_matches_filter(edge, selection))
        .collect::<Vec<_>>();
    if dependencies.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![match selection.capability.as_deref() {
        Some(capability) => format!("Dependencies (@{capability}):"),
        None => "Dependencies:".to_string(),
    }];
    for (index, edge) in dependencies.iter().enumerate() {
        lines.push(format!(
            "{:>2}. {} -> {}  [{}{}]",
            index + 1,
            edge.from,
            edge.to,
            edge.kind,
            if edge.blocking { ", blocking" } else { "" }
        ));
    }
    lines
}

fn dependency_matches_filter(
    edge: &crate::orchestration_registry::OrchestrationDependencyEdge,
    selection: &DependencySelection,
) -> bool {
    let filter_matches = match selection.filter {
        DependencyFilter::All => true,
        DependencyFilter::Blocking => edge.blocking,
        DependencyFilter::Sidecars => !edge.blocking,
        DependencyFilter::Missing => edge.kind == "dependsOnCapability:missing",
        DependencyFilter::Booting => edge.kind == "dependsOnCapability:booting",
        DependencyFilter::Ambiguous => edge.kind == "dependsOnCapability:ambiguous",
        DependencyFilter::Satisfied => edge.kind == "dependsOnCapability:satisfied",
    };
    if !filter_matches {
        return false;
    }
    match selection.capability.as_deref() {
        Some(capability) => edge.to == format!("capability:@{capability}"),
        None => true,
    }
}

fn render_live_agent_tasks_section(tasks: &[LiveAgentTaskSummary]) -> Vec<String> {
    let mut lines = vec!["Live agent tasks:".to_string()];
    for (index, task) in tasks.iter().enumerate() {
        let receiver_preview = if task.receiver_thread_ids.is_empty() {
            "-".to_string()
        } else {
            task.receiver_thread_ids.join(", ")
        };
        let status_preview = if task.agent_statuses.is_empty() {
            "-".to_string()
        } else {
            task.agent_statuses
                .iter()
                .map(|(thread_id, status)| format!("{thread_id}={status}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        lines.push(format!(
            "{:>2}. {}  [{}]  {} -> {}",
            index + 1,
            task.tool,
            task.status,
            task.sender_thread_id,
            receiver_preview
        ));
        lines.push(format!("    task     {}", task.id));
        lines.push(format!("    role     {}", task_role(task)));
        lines.push(format!(
            "    blocking {}",
            if task.tool == "wait" && task.status == "inProgress" {
                "yes"
            } else {
                "no"
            }
        ));
        lines.push(format!("    agents   {status_preview}"));
        if let Some(prompt) = task.prompt.as_deref() {
            lines.push(format!("    prompt   {prompt}"));
        }
    }
    lines
}

fn render_cached_agent_threads_section(agent_threads: &[CachedAgentThreadSummary]) -> Vec<String> {
    let mut lines = vec!["Cached agent threads:".to_string()];
    for (index, agent) in agent_threads.iter().enumerate() {
        let mut line = format!("{:>2}. {}  [{}]", index + 1, agent.id, agent.status);
        if let Some(updated_at) = agent.updated_at {
            line.push_str(&format!("  [updated {updated_at}]"));
        }
        if !agent.preview.is_empty() && agent.preview != "-" {
            line.push_str(&format!("  {}", agent.preview));
        }
        lines.push(line);
    }
    lines.push("Use :multi-agents to refresh or switch agent threads.".to_string());
    lines
}

fn render_server_background_terminals_only(state: &AppState) -> Option<Vec<String>> {
    let background = render_background_terminals(state);
    if background == "No background terminals running." {
        return None;
    }
    let lines = background
        .lines()
        .take_while(|line| *line != "Local background shell jobs:")
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if lines.is_empty() || lines[0] != "Server-observed background terminals:" {
        None
    } else {
        Some(lines)
    }
}

fn push_section_gap(lines: &mut Vec<String>) {
    if !lines.is_empty() {
        lines.push(String::new());
    }
}

fn empty_filter_message(filter: WorkerFilter) -> &'static str {
    match filter {
        WorkerFilter::All => {
            "No workers tracked yet. Use :multi-agents to cache agent threads or start a background task."
        }
        WorkerFilter::Blockers => "No blocking workers tracked right now.",
        WorkerFilter::Dependencies => "No dependency edges tracked right now.",
        WorkerFilter::Agents => "No agent workers tracked right now.",
        WorkerFilter::Shells => "No local background shell jobs tracked right now.",
        WorkerFilter::Services => "No service shells tracked right now.",
        WorkerFilter::Capabilities => "No reusable service capabilities tracked right now.",
        WorkerFilter::Terminals => "No server-observed background terminals tracked right now.",
        WorkerFilter::Guidance => "No orchestration guidance right now.",
        WorkerFilter::Actions => "No orchestration actions suggested right now.",
    }
}

fn empty_dependency_filter_message(selection: &DependencySelection) -> String {
    let base = match selection.filter {
        DependencyFilter::All => "No dependency edges tracked right now.",
        DependencyFilter::Blocking => "No blocking dependency edges tracked right now.",
        DependencyFilter::Sidecars => "No sidecar dependency edges tracked right now.",
        DependencyFilter::Missing => "No missing capability dependencies tracked right now.",
        DependencyFilter::Booting => "No booting capability dependencies tracked right now.",
        DependencyFilter::Ambiguous => "No ambiguous capability dependencies tracked right now.",
        DependencyFilter::Satisfied => "No satisfied capability dependencies tracked right now.",
    };
    match selection.capability.as_deref() {
        Some(capability) => format!("{base} Capability selector: @{capability}."),
        None => base.to_string(),
    }
}

pub(super) fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

pub(crate) fn summarize_agent_preview(preview: &str) -> String {
    summarize_text(preview)
}

#[cfg(test)]
mod tests;
