use std::collections::BTreeMap;

use crate::background_shells::BackgroundShellIntent;
use crate::background_terminals::render_background_terminals;
use crate::background_terminals::server_background_terminal_count;
use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::blocking_dependency_count;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::orchestration_dependency_edges;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::orchestration_registry::sidecar_dependency_count;
use crate::orchestration_registry::task_role;
use crate::orchestration_registry::wait_dependency_summary;
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
    pub(crate) live_agent_tasks: Vec<LiveAgentTaskSummary>,
    pub(crate) background_shell_jobs: usize,
    pub(crate) thread_background_terminals: usize,
}

pub(crate) fn orchestration_snapshot(state: &AppState) -> OrchestrationSnapshot {
    OrchestrationSnapshot {
        main_agents: 1,
        cached_agent_threads: state.cached_agent_threads.clone(),
        live_agent_tasks: live_agent_tasks(state),
        background_shell_jobs: state.background_shells.job_count(),
        thread_background_terminals: server_background_terminal_count(state),
    }
}

pub(crate) fn orchestration_overview_summary(state: &AppState) -> String {
    let snapshot = orchestration_snapshot(state);
    let agent_counts = summarize_agent_status_counts(&snapshot.cached_agent_threads);
    format!(
        "main={} deps_blocking={} deps_sidecar={} waits={} sidecar_agents={} exec_prereqs={} exec_sidecars={} exec_services={} agents_live={} agents_cached={}{} bg_shells={} thread_terms={}",
        snapshot.main_agents,
        blocking_dependency_count(state),
        sidecar_dependency_count(state),
        active_wait_task_count(state),
        active_sidecar_agent_task_count(state),
        running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite),
        running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
        running_shell_count_by_intent(state, BackgroundShellIntent::Service),
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
    Some(format!(
        "main={} deps_blocking={} deps_sidecar={} waits={} sidecar_agents={} exec_prereqs={} exec_sidecars={} exec_services={} agent_tasks={} shells={} thread_terms={} agents={}{}",
        main_agent_state_label(state),
        blocking_dependency_count(state),
        sidecar_dependency_count(state),
        active_wait_task_count(state),
        active_sidecar_agent_task_count(state),
        running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite),
        running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
        running_shell_count_by_intent(state, BackgroundShellIntent::Service),
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

pub(crate) fn render_orchestration_workers(state: &AppState) -> String {
    let background = render_background_terminals(state);
    let has_background = background != "No background terminals running.";
    let mut lines = Vec::new();
    let mut main_line = format!("Main agent state: {}", main_agent_state_label(state));
    if let Some(waiting_on) = wait_dependency_summary(state) {
        main_line.push_str(&format!(" | {waiting_on}"));
    }
    main_line.push_str(&format!(
        " | sidecar agents={} | exec prereqs={} | exec sidecars={} | exec services={} | deps blocking={} sidecar={}",
        active_sidecar_agent_task_count(state),
        running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite),
        running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
        running_shell_count_by_intent(state, BackgroundShellIntent::Service),
        blocking_dependency_count(state),
        sidecar_dependency_count(state)
    ));
    lines.push(main_line);
    let dependencies = orchestration_dependency_edges(state);
    if !dependencies.is_empty() {
        lines.push(String::new());
        lines.push("Dependencies:".to_string());
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
    }
    if !state.live_agent_tasks.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("Live agent tasks:".to_string());
        for (index, task) in live_agent_tasks(state).iter().enumerate() {
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
    }
    if !state.cached_agent_threads.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
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

fn live_agent_tasks(state: &AppState) -> Vec<LiveAgentTaskSummary> {
    let mut tasks = state.live_agent_tasks.values().cloned().collect::<Vec<_>>();
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

pub(crate) fn summarize_agent_preview(preview: &str) -> String {
    summarize_text(preview)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::orchestration_registry::LiveAgentTaskSummary;

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
        assert!(summary.contains("deps_blocking=0"));
        assert!(summary.contains("deps_sidecar=0"));
        assert!(summary.contains("waits=0"));
        assert!(summary.contains("sidecar_agents=0"));
        assert!(summary.contains("exec_prereqs=0"));
        assert!(summary.contains("exec_sidecars=0"));
        assert!(summary.contains("exec_services=0"));
        assert!(summary.contains("agents_live=0"));
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
            "sidecar agents=1 | exec prereqs=0 | exec sidecars=0 | exec services=0 | deps blocking=1 sidecar=1"
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
}
