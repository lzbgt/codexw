use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_terminals::render_background_terminals;
use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_registry::active_sidecar_agent_task_count;
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
mod summary;

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
pub(crate) use summary::orchestration_background_summary;
pub(crate) use summary::orchestration_overview_summary;
pub(crate) use summary::orchestration_prompt_suffix;
pub(crate) use summary::orchestration_runtime_summary;

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

pub(super) fn live_agent_tasks(state: &AppState) -> Vec<LiveAgentTaskSummary> {
    let mut tasks = state
        .orchestration
        .live_agent_tasks
        .values()
        .cloned()
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    tasks
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
