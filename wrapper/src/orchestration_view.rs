use std::collections::BTreeMap;

use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
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

pub(crate) fn orchestration_guidance_summary(state: &AppState) -> Option<String> {
    guidance_lines(state).first().cloned()
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
    parts.push("/ps to view".to_string());
    parts.push("/clean to close".to_string());
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

pub(crate) fn render_orchestration_guidance(state: &AppState) -> String {
    let lines = guidance_lines(state);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Next action:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_guidance_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = guidance_lines_for_capability(state, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Next action (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

pub(crate) fn render_orchestration_actions(state: &AppState) -> String {
    let lines = action_lines(state, ActionAudience::Operator);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Suggested actions:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_actions_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = action_lines_for_capability(state, ActionAudience::Operator, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Suggested actions (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

pub(crate) fn render_orchestration_actions_for_tool(state: &AppState) -> String {
    let lines = action_lines(state, ActionAudience::Tool);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Suggested actions:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_actions_for_tool_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = action_lines_for_capability(state, ActionAudience::Tool, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Suggested actions (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionAudience {
    Operator,
    Tool,
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
    lines.push("Use /multi-agents to refresh or switch agent threads.".to_string());
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
            "No workers tracked yet. Use /multi-agents to cache agent threads or start a background task."
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

fn guidance_lines(state: &AppState) -> Vec<String> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let blocking_capability_issues = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues();
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    let terminals = server_background_terminal_count(state);

    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Missing)
    {
        return vec![
            format!(
                "A blocking shell depends on missing service capability @{}.",
                issue.capability
            ),
            "Start or relabel the missing reusable service before waiting on the shell result.".to_string(),
            "Use /ps capabilities to inspect the live provider map and /ps blockers to inspect the blocked shell.".to_string(),
        ];
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        return vec![
            format!(
                "A blocking shell depends on ambiguous service capability @{}.",
                issue.capability
            ),
            "Resolve the conflicting reusable service role before relying on capability-based attachment.".to_string(),
            "Use /ps capabilities to inspect providers and consumers for that capability.".to_string(),
        ];
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        return vec![
            format!(
                "A blocking shell is waiting on booting service capability @{}.",
                issue.capability
            ),
            "Use /ps capabilities or /ps services to inspect the provider and readiness state."
                .to_string(),
            "Use :ps wait <job> when the booting service has a readiness contract.".to_string(),
        ];
    }
    if prereqs > 0 {
        return vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(prereqs, "prerequisite shell", "prerequisite shells")
            ),
            "Inspect /ps blockers to identify the gating job.".to_string(),
            "Use :ps wait <job> for services with readiness contracts or :ps poll <job> to inspect raw output.".to_string(),
        ];
    }
    if waits > 0 {
        return vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(waits, "agent wait", "agent waits")
            ),
            "Inspect /ps blockers to see the blocking agent dependencies.".to_string(),
            "Use /multi-agents to refresh or switch into the relevant agent thread.".to_string(),
        ];
    }
    if !capability_conflicts.is_empty() {
        let conflict_count = capability_conflicts.len();
        let first = &capability_conflicts[0].0;
        return vec![
            format!(
                "{} detected across service capabilities.",
                pluralize(conflict_count, "capability conflict is", "capability conflicts are")
            ),
            format!("Resolve ambiguous reuse targets such as @{first} before relying on capability-based attachment."),
            "Use /ps capabilities to inspect the ambiguous capability map and assign more specific capabilities.".to_string(),
        ];
    }
    if ready_services > 0 {
        return vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
            "Use /ps services to inspect attachment metadata and available recipes.".to_string(),
            "Use :ps attach <job> or :ps run <job> <recipe> to reuse the service directly."
                .to_string(),
        ];
    }
    if booting_services > 0 {
        return vec![
            format!(
                "{} still booting.",
                pluralize(booting_services, "service shell is", "service shells are")
            ),
            "Use /ps services to inspect readiness state and startup metadata.".to_string(),
            "Use :ps wait <job> [timeoutMs] when later work depends on service readiness."
                .to_string(),
        ];
    }
    if sidecar_agents + shell_sidecars > 0 {
        let sidecars = sidecar_agents + shell_sidecars;
        return vec![
            format!(
                "{} running without blocking the main agent.",
                pluralize(sidecars, "sidecar is", "sidecars are")
            ),
            "Continue independent work on the foreground thread.".to_string(),
            "Use /ps agents or /ps shells to inspect progress only when the result becomes relevant.".to_string(),
        ];
    }
    if terminals > 0 {
        return vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use /ps terminals to inspect them or /clean terminals to close them.".to_string(),
        ];
    }

    Vec::new()
}

fn guidance_lines_for_capability(
    state: &AppState,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(issue) = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)
    {
        return Ok(match issue.status {
            BackgroundShellCapabilityDependencyState::Missing => vec![
                format!("A blocking shell depends on missing service capability @{capability}."),
                "Start or relabel the missing reusable service before waiting on the shell result."
                    .to_string(),
                format!(
                    "Use /ps capabilities @{capability} and /ps dependencies missing @{capability} to inspect the exact blocker."
                ),
            ],
            BackgroundShellCapabilityDependencyState::Ambiguous => vec![
                format!("A blocking shell depends on ambiguous service capability @{capability}."),
                "Resolve the conflicting reusable service role before relying on capability-based attachment.".to_string(),
                format!(
                    "Use /ps capabilities @{capability} and /ps services @{capability} to inspect the conflicting providers."
                ),
            ],
            BackgroundShellCapabilityDependencyState::Booting => vec![
                format!("A blocking shell is waiting on booting service capability @{capability}."),
                format!("Use /ps services booting @{capability} to inspect the provider and readiness state."),
                format!("Use :ps wait @{capability} 5000 when later work depends on readiness."),
            ],
            BackgroundShellCapabilityDependencyState::Satisfied => vec![],
        });
    }

    Ok(
        match state
            .orchestration
            .background_shells
            .service_capability_issue_for_ref(capability)?
        {
            BackgroundShellCapabilityIssueClass::Missing => {
                vec![
            format!("Reusable service capability @{capability} has no running provider."),
            "Start or relabel a service shell so later work can attach to that reusable role."
                .to_string(),
            format!("Use /ps capabilities @{capability} to confirm the missing-provider state."),
        ]
            }
            BackgroundShellCapabilityIssueClass::Ambiguous => vec![
                format!("Reusable service capability @{capability} is ambiguous."),
                "Resolve the conflicting providers before relying on capability-based reuse."
                    .to_string(),
                format!("Use /ps capabilities @{capability} to inspect providers and consumers."),
            ],
            BackgroundShellCapabilityIssueClass::Booting => vec![
                format!("Reusable service capability @{capability} is still booting."),
                format!("Use /ps services booting @{capability} to inspect provider readiness."),
                format!("Use :ps wait @{capability} 5000 when later work depends on readiness."),
            ],
            BackgroundShellCapabilityIssueClass::Healthy => vec![
                format!("Reusable service capability @{capability} is ready for reuse."),
                format!("Use /ps attach @{capability} to inspect endpoint and recipe details."),
                format!("Use :ps run @{capability} <recipe> [json-args] to reuse it directly."),
            ],
        },
    )
}

fn action_lines(state: &AppState, audience: ActionAudience) -> Vec<String> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let blocking_capability_issues = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues();
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    let terminals = server_background_terminal_count(state);

    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Missing)
    {
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps capabilities @{}` to inspect the missing provider map.",
                    issue.capability
                ),
                format!(
                    "Run `:ps dependencies missing @{}` to inspect the blocked dependency edges.",
                    issue.capability
                ),
                "If the blocked shell is no longer needed, run `:clean blockers`.".to_string(),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the missing provider map.",
                    issue.capability
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{}\"}}` to inspect the blocked dependency edges.",
                    issue.capability
                ),
                "Use `background_shell_clean {\"scope\":\"blockers\"}` to abandon the blocking prerequisite shells if they are no longer needed.".to_string(),
            ],
        };
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps capabilities @{}` to inspect the ambiguous provider set.",
                    issue.capability
                ),
                format!(
                    "Run `:clean services @{}` to clear the conflicting reusable role in one step.",
                    issue.capability
                ),
                format!(
                    "Run `:ps services @{}` to inspect the remaining providers.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the ambiguous provider set.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{}\"}}` to clear the conflicting reusable role in one step.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{}\"}}` to inspect the remaining providers.",
                    issue.capability
                ),
            ],
        };
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps services @{}` to inspect the booting provider state.",
                    issue.capability
                ),
                format!(
                    "Run `:ps wait @{} 5000` to wait on the capability provider.",
                    issue.capability
                ),
                format!(
                    "Run `:ps dependencies booting @{}` to keep the dependency view focused.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{}\"}}` to inspect the booting provider state.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"@{}\",\"timeoutMs\":5000}}` to wait on the capability provider.",
                    issue.capability
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"booting\",\"capability\":\"@{}\"}}` to keep the dependency view focused.",
                    issue.capability
                ),
            ],
        };
    }
    if prereqs > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps blockers` to inspect the gating shell or wait dependency.".to_string(),
                "Run `:ps poll <job>` or `:ps wait <job> [timeoutMs]` on the blocker you care about."
                    .to_string(),
                "Run `:clean blockers` to abandon the current blocking prerequisite work.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the gating shell or wait dependency.".to_string(),
                "Use `background_shell_poll {\"jobId\":\"bg-...\"}` or `background_shell_wait_ready {\"jobId\":\"@capability\",\"timeoutMs\":5000}` on the blocker you care about."
                    .to_string(),
                "Use `background_shell_clean {\"scope\":\"blockers\"}` to abandon the current blocking prerequisite work.".to_string(),
            ],
        };
    }
    if waits > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps blockers` to inspect the active wait dependencies.".to_string(),
                "Run `:multi-agents` to refresh spawned agent threads.".to_string(),
                "Run `:resume <n>` to switch into the agent thread that matters.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the active wait dependencies.".to_string(),
                "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect cached and live agent workers.".to_string(),
                "Continue foreground work until one of the waiting agent results becomes critical.".to_string(),
            ],
        };
    }
    if let Some((capability, _)) = capability_conflicts.first() {
        return match audience {
            ActionAudience::Operator => vec![
                format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
                format!(
                    "Run `:clean services @{capability}` to clear the ambiguous reusable role."
                ),
                format!("Run `:ps services @{capability}` to verify the surviving providers."),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the ambiguous reusable role."
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to verify the surviving providers."
                ),
            ],
        };
    }
    if ready_services > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services ready` to inspect reusable service metadata.".to_string(),
                "Run `:ps attach <job|@capability>` to inspect endpoint and recipe details.".to_string(),
                "Run `:ps run <job|@capability> <recipe> [json-args]` to reuse the ready service directly."
                    .to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"ready\"}` to inspect reusable service metadata.".to_string(),
                "Use `background_shell_attach {\"jobId\":\"@capability\"}` to inspect endpoint and recipe details for the service you choose.".to_string(),
                "Use `background_shell_invoke_recipe {\"jobId\":\"@capability\",\"recipe\":\"...\"}` to reuse the ready service directly.".to_string(),
            ],
        };
    }
    if booting_services > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services booting` to inspect readiness state and startup metadata."
                    .to_string(),
                "Run `:ps wait <job|@capability> [timeoutMs]` for the booting service you need."
                    .to_string(),
                "Run `:ps capabilities booting` to keep the capability view focused.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"booting\"}` to inspect readiness state and startup metadata.".to_string(),
                "Use `background_shell_wait_ready {\"jobId\":\"@capability\",\"timeoutMs\":5000}` for the booting service you need.".to_string(),
                "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
            ],
        };
    }
    if sidecar_agents + shell_sidecars > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps agents` to inspect sidecar agent progress.".to_string(),
                "Run `:ps shells` to inspect non-blocking shell jobs.".to_string(),
                "Continue foreground work until one of those results becomes relevant.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect sidecar agent progress.".to_string(),
                "Use `orchestration_list_workers {\"filter\":\"shells\"}` to inspect non-blocking shell jobs.".to_string(),
                "Continue foreground work until one of those results becomes relevant.".to_string(),
            ],
        };
    }
    if terminals > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps terminals` to inspect server-observed background terminals.".to_string(),
                "Run `:clean terminals` to close them if they are no longer needed.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"terminals\"}` to inspect server-observed background terminals.".to_string(),
                "Terminal cleanup is operator-only; use `:clean terminals` from the wrapper when they are no longer needed.".to_string(),
            ],
        };
    }

    Vec::new()
}

fn action_lines_for_capability(
    state: &AppState,
    audience: ActionAudience,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(issue) = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)
    {
        return Ok(match (issue.status, audience) {
            (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Operator) => vec![
                format!("Run `:ps capabilities @{capability}` to inspect the missing provider map."),
                format!(
                    "Run `:ps dependencies missing @{capability}` to inspect the blocked dependency edges."
                ),
                "If the blocked shell is no longer needed, run `:clean blockers`.".to_string(),
            ],
            (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Tool) => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the missing provider map."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{capability}\"}}` to inspect the blocked dependency edges."
                ),
                "Use `background_shell_clean {\"scope\":\"blockers\"}` to abandon the blocking prerequisite shells if they are no longer needed.".to_string(),
            ],
            (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Operator) => vec![
                format!("Run `:ps capabilities @{capability}` to inspect the ambiguous provider set."),
                format!(
                    "Run `:clean services @{capability}` to clear the conflicting reusable role in one step."
                ),
                format!("Run `:ps services @{capability}` to inspect the remaining providers."),
            ],
            (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Tool) => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the ambiguous provider set."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the conflicting reusable role in one step."
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to inspect the remaining providers."
                ),
            ],
            (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Operator) => vec![
                format!("Run `:ps services booting @{capability}` to inspect the booting provider state."),
                format!("Run `:ps wait @{capability} 5000` to wait on the capability provider."),
                format!(
                    "Run `:ps dependencies booting @{capability}` to keep the dependency view focused."
                ),
            ],
            (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Tool) => vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect the booting provider state."
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"@{capability}\",\"timeoutMs\":5000}}` to wait on the capability provider."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"booting\",\"capability\":\"@{capability}\"}}` to keep the dependency view focused."
                ),
            ],
            (BackgroundShellCapabilityDependencyState::Satisfied, _) => vec![],
        });
    }

    Ok(match (
        state.orchestration.background_shells.service_capability_issue_for_ref(capability)?,
        audience,
    ) {
        (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Operator) => vec![
            format!("Run `:ps capabilities @{capability}` to confirm there is no running provider."),
            format!("Start or relabel a service shell so it provides `@{capability}`."),
        ],
        (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Tool) => vec![
            format!(
                "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to confirm there is no running provider."
            ),
            "Start or relabel a reusable service so it provides that capability before attaching future work to it.".to_string(),
        ],
        (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Operator) => vec![
            format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
            format!("Run `:clean services @{capability}` to clear the ambiguous reusable role."),
            format!("Run `:ps services @{capability}` to verify the surviving providers."),
        ],
        (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Tool) => vec![
            format!(
                "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
            ),
            format!(
                "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the ambiguous reusable role."
            ),
            format!(
                "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to verify the surviving providers."
            ),
        ],
        (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Operator) => vec![
            format!("Run `:ps services booting @{capability}` to inspect provider readiness."),
            format!("Run `:ps wait @{capability} 5000` for the booting service you need."),
            "Run `:ps capabilities booting` to keep the capability view focused.".to_string(),
        ],
        (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Tool) => vec![
            format!(
                "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
            ),
            format!(
                "Use `background_shell_wait_ready {{\"jobId\":\"@{capability}\",\"timeoutMs\":5000}}` for the booting service you need."
            ),
            "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
        ],
        (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Operator) => vec![
            format!("Run `:ps attach @{capability}` to inspect endpoint and recipe details."),
            format!("Run `:ps run @{capability} <recipe> [json-args]` to reuse the ready service directly."),
        ],
        (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Tool) => vec![
            format!(
                "Use `background_shell_attach {{\"jobId\":\"@{capability}\"}}` to inspect endpoint and recipe details."
            ),
            format!(
                "Use `background_shell_invoke_recipe {{\"jobId\":\"@{capability}\",\"recipe\":\"...\"}}` to reuse the ready service directly."
            ),
        ],
    })
}

fn normalize_capability_ref(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().trim_start_matches('@');
    if normalized.is_empty() {
        return Err("capability selector must be a non-empty capability name".to_string());
    }
    Ok(normalized.to_string())
}

fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

pub(crate) fn summarize_agent_preview(preview: &str) -> String {
    summarize_text(preview)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::orchestration_registry::LiveAgentTaskSummary;

    use super::CachedAgentThreadSummary;
    use super::WorkerFilter;
    use super::orchestration_background_summary;
    use super::orchestration_guidance_summary;
    use super::orchestration_overview_summary;
    use super::orchestration_prompt_suffix;
    use super::orchestration_runtime_summary;
    use super::render_orchestration_actions;
    use super::render_orchestration_actions_for_capability;
    use super::render_orchestration_actions_for_tool;
    use super::render_orchestration_actions_for_tool_capability;
    use super::render_orchestration_guidance;
    use super::render_orchestration_guidance_for_capability;
    use super::render_orchestration_workers;
    use super::render_orchestration_workers_with_filter;

    #[test]
    fn orchestration_summary_includes_agent_status_breakdown() {
        let mut state = crate::state::AppState::new(true, false);
        state.cached_agent_threads = vec![
            CachedAgentThreadSummary {
                id: "agent-1".to_string(),
                status: "active".to_string(),
                preview: "inspect auth".to_string(),
                updated_at: Some(10),
            },
            CachedAgentThreadSummary {
                id: "agent-2".to_string(),
                status: "idle".to_string(),
                preview: "review API".to_string(),
                updated_at: Some(5),
            },
        ];
        let summary = orchestration_overview_summary(&state);
        assert!(summary.contains("main=1"));
        assert!(summary.contains("deps_blocking=0"));
        assert!(summary.contains("deps_sidecar=0"));
        assert!(summary.contains("waits=0"));
        assert!(summary.contains("sidecar_agents=0"));
        assert!(summary.contains("exec_prereqs=0"));
        assert!(summary.contains("exec_sidecars=0"));
        assert!(summary.contains("exec_services=0"));
        assert!(summary.contains("services_ready=0"));
        assert!(summary.contains("services_booting=0"));
        assert!(summary.contains("services_untracked=0"));
        assert!(summary.contains("services_conflicted=0"));
        assert!(summary.contains("service_caps=0"));
        assert!(summary.contains("service_cap_conflicts=0"));
        assert!(summary.contains("agents_live=0"));
        assert!(summary.contains("agents_cached=2"));
        assert!(summary.contains("active=1"));
        assert!(summary.contains("idle=1"));
    }

    #[test]
    fn orchestration_runtime_summary_is_empty_when_no_workers_exist() {
        let state = crate::state::AppState::new(true, false);
        assert!(orchestration_runtime_summary(&state).is_none());
        assert!(orchestration_prompt_suffix(&state).is_none());
        assert!(orchestration_background_summary(&state).is_none());
        assert!(orchestration_guidance_summary(&state).is_none());
    }

    #[test]
    fn orchestration_prompt_suffix_distinguishes_blockers_sidecars_services_and_terminals() {
        let mut state = crate::state::AppState::new(true, false);
        state.thread_id = Some("thread-main".to_string());
        state.live_agent_tasks.insert(
            "call-1".to_string(),
            LiveAgentTaskSummary {
                id: "call-1".to_string(),
                tool: "wait".to_string(),
                status: "inProgress".to_string(),
                sender_thread_id: "thread-main".to_string(),
                receiver_thread_ids: vec!["agent-1".to_string()],
                prompt: None,
                agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
            },
        );
        state
            .background_shells
            .start_from_tool(
                &serde_json::json!({"command": "sleep 0.4", "intent": "prerequisite"}),
                "/tmp",
            )
            .expect("start prerequisite shell");
        state
            .background_shells
            .start_from_tool(
                &serde_json::json!({"command": "sleep 0.4", "intent": "service"}),
                "/tmp",
            )
            .expect("start service shell");
        state.background_terminals.insert(
            "proc-1".to_string(),
            crate::background_terminals::BackgroundTerminalSummary {
                item_id: "cmd-1".to_string(),
                process_id: "proc-1".to_string(),
                command_display: "python worker.py".to_string(),
                waiting: true,
                recent_inputs: Vec::new(),
                recent_output: vec!["ready".to_string()],
            },
        );

        let suffix = orchestration_prompt_suffix(&state).expect("prompt suffix");
        assert!(suffix.contains("blocked on 1 agent wait and 1 prerequisite shell"));
        assert!(suffix.contains("1 service untracked"));
        assert!(suffix.contains("1 terminal"));
        assert!(suffix.contains("/ps to view"));
        let background = orchestration_background_summary(&state).expect("background summary");
        assert!(background.contains("prereqs=1"));
        assert!(background.contains("shell_sidecars=0"));
        assert!(background.contains("services=1"));
        assert!(background.contains("services_untracked=1"));
        assert!(background.contains("terminals=1"));
        let _ = state.background_shells.terminate_all_running();
    }

    #[test]
    fn orchestration_worker_rendering_includes_cached_agents_and_background_tasks() {
        let mut state = crate::state::AppState::new(true, false);
        state.live_agent_tasks.insert(
            "call-1".to_string(),
            LiveAgentTaskSummary {
                id: "call-1".to_string(),
                tool: "spawnAgent".to_string(),
                status: "inProgress".to_string(),
                sender_thread_id: "thread-main".to_string(),
                receiver_thread_ids: vec!["agent-1".to_string()],
                prompt: Some("inspect auth".to_string()),
                agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
            },
        );
        state.live_agent_tasks.insert(
            "call-2".to_string(),
            LiveAgentTaskSummary {
                id: "call-2".to_string(),
                tool: "wait".to_string(),
                status: "inProgress".to_string(),
                sender_thread_id: "thread-main".to_string(),
                receiver_thread_ids: vec!["agent-1".to_string()],
                prompt: None,
                agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
            },
        );
        state.cached_agent_threads = vec![CachedAgentThreadSummary {
            id: "agent-1".to_string(),
            status: "active".to_string(),
            preview: "inspect auth".to_string(),
            updated_at: Some(10),
        }];
        state.background_terminals.insert(
            "proc-1".to_string(),
            crate::background_terminals::BackgroundTerminalSummary {
                item_id: "cmd-1".to_string(),
                process_id: "proc-1".to_string(),
                command_display: "python worker.py".to_string(),
                waiting: true,
                recent_inputs: Vec::new(),
                recent_output: vec!["ready".to_string()],
            },
        );

        let rendered = render_orchestration_workers(&state);
        assert!(rendered.contains("Main agent state: blocked | waiting on agent agent-1"));
        assert!(rendered.contains(
            "sidecar agents=1 | exec prereqs=0 | exec sidecars=0 | exec services=0 (ready=0 booting=0 untracked=0 conflicted=0) | deps blocking=1 sidecar=1"
        ));
        assert!(rendered.contains("Dependencies:"));
        assert!(rendered.contains("main -> agent:agent-1  [wait, blocking]"));
        assert!(rendered.contains("main -> agent:agent-1  [spawnAgent]"));
        assert!(rendered.contains("Live agent tasks:"));
        assert!(rendered.contains("spawnAgent  [inProgress]  thread-main -> agent-1"));
        assert!(rendered.contains("wait  [inProgress]  thread-main -> agent-1"));
        assert!(rendered.contains("role     sidecar"));
        assert!(rendered.contains("role     blocked"));
        assert!(rendered.contains("blocking yes"));
        assert!(rendered.contains("Cached agent threads:"));
        assert!(rendered.contains("agent-1  [active]"));
        assert!(rendered.contains("inspect auth"));
        assert!(rendered.contains("Use /multi-agents to refresh or switch agent threads."));
        assert!(rendered.contains("Server-observed background terminals:"));
        assert!(rendered.contains("python worker.py"));
    }

    #[test]
    fn filtered_worker_rendering_can_target_blockers_services_and_terminals() {
        let mut state = crate::state::AppState::new(true, false);
        state.thread_id = Some("thread-main".to_string());
        state.live_agent_tasks.insert(
            "call-wait".to_string(),
            LiveAgentTaskSummary {
                id: "call-wait".to_string(),
                tool: "wait".to_string(),
                status: "inProgress".to_string(),
                sender_thread_id: "thread-main".to_string(),
                receiver_thread_ids: vec!["agent-1".to_string()],
                prompt: None,
                agent_statuses: BTreeMap::from([("agent-1".to_string(), "running".to_string())]),
            },
        );
        state.cached_agent_threads = vec![CachedAgentThreadSummary {
            id: "agent-1".to_string(),
            status: "active".to_string(),
            preview: "inspect auth".to_string(),
            updated_at: Some(10),
        }];
        state
            .background_shells
            .start_from_tool(
                &serde_json::json!({"command": "sleep 0.4", "intent": "prerequisite"}),
                "/tmp",
            )
            .expect("start prerequisite shell");
        state
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "label": "dev server",
                    "capabilities": ["frontend.dev"],
                    "protocol": "http",
                    "endpoint": "http://127.0.0.1:3000",
                    "attachHint": "Open the dev server in a browser",
                    "recipes": [
                        {
                            "name": "health",
                            "description": "Check health"
                        }
                    ]
                }),
                "/tmp",
            )
            .expect("start service shell");
        state.background_terminals.insert(
            "proc-1".to_string(),
            crate::background_terminals::BackgroundTerminalSummary {
                item_id: "cmd-1".to_string(),
                process_id: "proc-1".to_string(),
                command_display: "python worker.py".to_string(),
                waiting: true,
                recent_inputs: Vec::new(),
                recent_output: vec!["ready".to_string()],
            },
        );

        let blockers = render_orchestration_workers_with_filter(&state, WorkerFilter::Blockers);
        assert!(blockers.contains("Dependencies:"));
        assert!(blockers.contains("wait, blocking"));
        assert!(blockers.contains("backgroundShell:prerequisite, blocking"));
        assert!(!blockers.contains("Cached agent threads:"));
        assert!(!blockers.contains("Server-observed background terminals:"));

        let dependencies =
            render_orchestration_workers_with_filter(&state, WorkerFilter::Dependencies);
        assert!(dependencies.contains("Dependencies:"));
        assert!(!dependencies.contains("Main agent state:"));
        assert!(dependencies.contains("main -> agent:agent-1  [wait, blocking]"));
        assert!(
            dependencies.contains("main -> shell:bg-1  [backgroundShell:prerequisite, blocking]")
        );

        let services = render_orchestration_workers_with_filter(&state, WorkerFilter::Services);
        assert!(services.contains("Local background shell jobs:"));
        assert!(services.contains("intent   service"));
        assert!(services.contains("label    dev server"));
        assert!(services.contains("protocol http"));
        assert!(services.contains("endpoint http://127.0.0.1:3000"));
        assert!(services.contains("attach   Open the dev server in a browser"));
        assert!(services.contains("recipes  1"));
        assert!(services.contains("service  untracked"));
        assert!(services.contains("Capability index:"));
        assert!(!services.contains("intent   prerequisite"));

        let capabilities =
            render_orchestration_workers_with_filter(&state, WorkerFilter::Capabilities);
        assert!(capabilities.contains("Service capability index:"));
        assert!(!capabilities.contains("Local background shell jobs:"));

        let terminals = render_orchestration_workers_with_filter(&state, WorkerFilter::Terminals);
        assert!(terminals.contains("Server-observed background terminals:"));
        assert!(terminals.contains("python worker.py"));
        assert!(!terminals.contains("Local background shell jobs:"));
        let _ = state.background_shells.terminate_all_running();
    }

    #[test]
    fn orchestration_service_counts_distinguish_ready_booting_and_untracked() {
        let state = crate::state::AppState::new(true, false);
        state
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": if cfg!(windows) {
                        "echo READY && ping -n 2 127.0.0.1 >NUL"
                    } else {
                        "printf 'READY\\n'; sleep 0.4"
                    },
                    "intent": "service",
                    "readyPattern": "READY"
                }),
                "/tmp",
            )
            .expect("start ready service");
        state
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "readyPattern": "READY"
                }),
                "/tmp",
            )
            .expect("start booting service");
        state
            .background_shells
            .start_from_tool(
                &serde_json::json!({"command": "sleep 0.4", "intent": "service"}),
                "/tmp",
            )
            .expect("start untracked service");

        for _ in 0..40 {
            let summary = orchestration_background_summary(&state).expect("background summary");
            if summary.contains("services_ready=1") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        let summary = orchestration_overview_summary(&state);
        assert!(summary.contains("exec_services=3"));
        assert!(summary.contains("services_ready=1"));
        assert!(summary.contains("services_booting=1"));
        assert!(summary.contains("services_untracked=1"));
        assert!(summary.contains("services_conflicted=0"));
        assert!(summary.contains("service_caps=0"));
        assert!(summary.contains("service_cap_conflicts=0"));

        let suffix = orchestration_prompt_suffix(&state).expect("prompt suffix");
        assert!(suffix.contains("1 service ready"));
        assert!(suffix.contains("1 service booting"));
        assert!(suffix.contains("1 service untracked"));
        let _ = state.background_shells.terminate_all_running();
    }

    #[test]
    fn orchestration_guidance_prefers_blockers_then_ready_services_then_sidecars() {
        let blocked = crate::state::AppState::new(true, false);
        blocked
            .background_shells
            .start_from_tool(
                &serde_json::json!({"command": "sleep 0.4", "intent": "prerequisite"}),
                "/tmp",
            )
            .expect("start prerequisite shell");
        let blocked_hint = orchestration_guidance_summary(&blocked).expect("blocked guidance");
        assert!(blocked_hint.contains("blocked on 1 prerequisite shell"));
        let blocked_view = render_orchestration_guidance(&blocked);
        assert!(blocked_view.contains("Inspect /ps blockers"));
        let _ = blocked.background_shells.terminate_all_running();

        let ready = crate::state::AppState::new(true, false);
        ready
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": if cfg!(windows) {
                        "echo READY && ping -n 2 127.0.0.1 >NUL"
                    } else {
                        "printf 'READY\\n'; sleep 0.4"
                    },
                    "intent": "service",
                    "readyPattern": "READY"
                }),
                "/tmp",
            )
            .expect("start ready service");
        for _ in 0..40 {
            if let Some(hint) = orchestration_guidance_summary(&ready)
                && hint.contains("ready for reuse")
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let ready_hint = orchestration_guidance_summary(&ready).expect("ready guidance");
        assert!(ready_hint.contains("ready for reuse"));
        let _ = ready.background_shells.terminate_all_running();

        let mut sidecar = crate::state::AppState::new(true, false);
        sidecar.live_agent_tasks.insert(
            "call-1".to_string(),
            LiveAgentTaskSummary {
                id: "call-1".to_string(),
                tool: "spawnAgent".to_string(),
                status: "inProgress".to_string(),
                sender_thread_id: "thread-main".to_string(),
                receiver_thread_ids: vec!["agent-1".to_string()],
                prompt: None,
                agent_statuses: BTreeMap::new(),
            },
        );
        let sidecar_hint = orchestration_guidance_summary(&sidecar).expect("sidecar guidance");
        assert!(sidecar_hint.contains("running without blocking"));
    }

    #[test]
    fn orchestration_guidance_surfaces_service_capability_conflicts_before_ready_reuse() {
        let services = crate::state::AppState::new(true, false);
        services
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "capabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start first service");
        services
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "capabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start second service");
        let hint = orchestration_guidance_summary(&services).expect("conflict guidance");
        assert!(hint.contains("capability conflict"));
        let rendered = render_orchestration_guidance(&services);
        assert!(rendered.contains("@api.http"));
        assert!(rendered.contains("/ps capabilities"));
        let _ = services.background_shells.terminate_all_running();
    }

    #[test]
    fn orchestration_guidance_prioritizes_missing_blocking_service_dependencies() {
        let blocked = crate::state::AppState::new(true, false);
        blocked
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "prerequisite",
                    "dependsOnCapabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start dependent shell");
        let hint = orchestration_guidance_summary(&blocked).expect("dependency guidance");
        assert!(hint.contains("missing service capability @api.http"));
        let rendered = render_orchestration_guidance(&blocked);
        assert!(rendered.contains("/ps capabilities"));
        assert!(rendered.contains("/ps blockers"));
        let blockers = render_orchestration_workers_with_filter(&blocked, WorkerFilter::Blockers);
        assert!(blockers.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        ));
        let _ = blocked.background_shells.terminate_all_running();
    }

    #[test]
    fn guidance_filter_renders_next_action_section() {
        let mut state = crate::state::AppState::new(true, false);
        state.live_agent_tasks.insert(
            "call-1".to_string(),
            LiveAgentTaskSummary {
                id: "call-1".to_string(),
                tool: "wait".to_string(),
                status: "inProgress".to_string(),
                sender_thread_id: "thread-main".to_string(),
                receiver_thread_ids: vec!["agent-1".to_string()],
                prompt: None,
                agent_statuses: BTreeMap::new(),
            },
        );
        let rendered = render_orchestration_workers_with_filter(&state, WorkerFilter::Guidance);
        assert!(rendered.contains("Next action:"));
        assert!(rendered.contains("blocked on 1 agent wait"));
        assert!(rendered.contains("/multi-agents"));
    }

    #[test]
    fn actions_filter_renders_suggested_commands_for_conflicted_services() {
        let services = crate::state::AppState::new(true, false);
        services
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "capabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start first service");
        services
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "capabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start second service");

        let rendered = render_orchestration_actions(&services);
        assert!(rendered.contains("Suggested actions:"));
        assert!(rendered.contains(":ps capabilities @api.http"));
        assert!(rendered.contains(":clean services @api.http"));

        let tool_rendered = render_orchestration_actions_for_tool(&services);
        assert!(tool_rendered.contains("Suggested actions:"));
        assert!(
            tool_rendered
                .contains("background_shell_inspect_capability {\"capability\":\"@api.http\"}")
        );
        assert!(tool_rendered.contains(
            "background_shell_clean {\"scope\":\"services\",\"capability\":\"@api.http\"}"
        ));

        let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
        assert!(filtered.contains("Suggested actions:"));
        assert!(filtered.contains(":ps capabilities @api.http"));
        let _ = services.background_shells.terminate_all_running();
    }

    #[test]
    fn focused_guidance_and_actions_can_target_one_capability() {
        let services = crate::state::AppState::new(true, false);
        services
            .background_shells
            .start_from_tool(
                &serde_json::json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "capabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start service");

        let guidance = render_orchestration_guidance_for_capability(&services, "@api.http")
            .expect("focused guidance");
        assert!(guidance.contains("Next action (@api.http):"));
        assert!(guidance.contains("ready for reuse"));

        let operator_actions = render_orchestration_actions_for_capability(&services, "@api.http")
            .expect("focused operator actions");
        assert!(operator_actions.contains("Suggested actions (@api.http):"));
        assert!(operator_actions.contains(":ps attach @api.http"));

        let tool_actions = render_orchestration_actions_for_tool_capability(&services, "@api.http")
            .expect("focused tool actions");
        assert!(tool_actions.contains("Suggested actions (@api.http):"));
        assert!(tool_actions.contains("background_shell_attach {\"jobId\":\"@api.http\"}"));
        let _ = services.background_shells.terminate_all_running();
    }
}
