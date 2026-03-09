mod agents;
mod background;
mod main_agent;

use crate::background_shells::BackgroundShellIntent;
use crate::state::AppState;

use super::WorkerFilter;
pub(crate) use agents::live_agent_tasks;
use agents::render_cached_agent_threads_section;
use agents::render_live_agent_tasks_section;
use background::render_server_background_terminals_only;
use main_agent::render_main_agent_section;

pub(crate) fn render_orchestration_workers(state: &AppState) -> String {
    render_orchestration_workers_with_filter(state, WorkerFilter::All)
}

pub(crate) fn render_orchestration_workers_with_filter(
    state: &AppState,
    filter: WorkerFilter,
) -> String {
    if matches!(filter, WorkerFilter::Guidance) {
        let guidance = super::render_orchestration_guidance(state);
        if guidance.is_empty() {
            return empty_filter_message(filter).to_string();
        }
        return guidance;
    }
    if matches!(filter, WorkerFilter::Actions) {
        let actions = super::render_orchestration_actions(state);
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
