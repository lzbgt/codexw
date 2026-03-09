use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::state::summarize_text;

mod dependencies;
mod guidance_actions;
mod summary;
mod workers;

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
pub(crate) use workers::render_orchestration_workers;
pub(crate) use workers::render_orchestration_workers_with_filter;

pub(crate) use dependencies::render_orchestration_dependencies;

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

pub(super) fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

pub(super) fn live_agent_tasks(state: &crate::state::AppState) -> Vec<LiveAgentTaskSummary> {
    workers::live_agent_tasks(state)
}

pub(crate) fn summarize_agent_preview(preview: &str) -> String {
    summarize_text(preview)
}

#[cfg(test)]
mod tests;
