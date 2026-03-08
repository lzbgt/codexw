use std::collections::BTreeMap;

use crate::background_terminals::render_background_terminals;
use crate::background_terminals::server_background_terminal_count;
use crate::state::AppState;
use crate::state::summarize_text;

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
    pub(crate) background_shell_jobs: usize,
    pub(crate) thread_background_terminals: usize,
}

pub(crate) fn orchestration_snapshot(state: &AppState) -> OrchestrationSnapshot {
    OrchestrationSnapshot {
        main_agents: 1,
        cached_agent_threads: state.cached_agent_threads.clone(),
        background_shell_jobs: state.background_shells.job_count(),
        thread_background_terminals: server_background_terminal_count(state),
    }
}

pub(crate) fn orchestration_overview_summary(state: &AppState) -> String {
    let snapshot = orchestration_snapshot(state);
    let agent_counts = summarize_agent_status_counts(&snapshot.cached_agent_threads);
    format!(
        "main={} agents_cached={}{} bg_shells={} thread_terms={}",
        snapshot.main_agents,
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
    if snapshot.background_shell_jobs == 0
        && snapshot.thread_background_terminals == 0
        && snapshot.cached_agent_threads.is_empty()
    {
        return None;
    }
    let agent_counts = summarize_agent_status_counts(&snapshot.cached_agent_threads);
    Some(format!(
        "shells={} thread_terms={} agents={}{}",
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

pub(crate) fn render_orchestration_workers(state: &AppState) -> String {
    let background = render_background_terminals(state);
    let has_background = background != "No background terminals running.";
    let mut lines = Vec::new();
    if !state.cached_agent_threads.is_empty() {
        lines.push("Cached agent threads:".to_string());
        for (index, agent) in state.cached_agent_threads.iter().enumerate() {
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
    }
    if has_background {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(background.lines().map(ToOwned::to_owned));
    }
    if lines.is_empty() {
        return "No workers tracked yet. Use /multi-agents to cache agent threads or start a background task.".to_string();
    }
    lines.join("\n")
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

pub(crate) fn summarize_agent_preview(preview: &str) -> String {
    summarize_text(preview)
}

#[cfg(test)]
mod tests {
    use super::CachedAgentThreadSummary;
    use super::orchestration_overview_summary;
    use super::orchestration_runtime_summary;
    use super::render_orchestration_workers;

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
        assert!(summary.contains("agents_cached=2"));
        assert!(summary.contains("active=1"));
        assert!(summary.contains("idle=1"));
    }

    #[test]
    fn orchestration_runtime_summary_is_empty_when_no_workers_exist() {
        let state = crate::state::AppState::new(true, false);
        assert!(orchestration_runtime_summary(&state).is_none());
    }

    #[test]
    fn orchestration_worker_rendering_includes_cached_agents_and_background_tasks() {
        let mut state = crate::state::AppState::new(true, false);
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
        assert!(rendered.contains("Cached agent threads:"));
        assert!(rendered.contains("agent-1  [active]"));
        assert!(rendered.contains("inspect auth"));
        assert!(rendered.contains("Use /multi-agents to refresh or switch agent threads."));
        assert!(rendered.contains("Server-observed background terminals:"));
        assert!(rendered.contains("python worker.py"));
    }
}
