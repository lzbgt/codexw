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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerFilter {
    All,
    Blockers,
    Agents,
    Shells,
    Services,
    Terminals,
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

pub(crate) fn orchestration_prompt_suffix(state: &AppState) -> Option<String> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecars = active_sidecar_agent_task_count(state)
        + running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let services = running_shell_count_by_intent(state, BackgroundShellIntent::Service);
    let terminals = server_background_terminal_count(state);
    if waits == 0 && prereqs == 0 && sidecars == 0 && services == 0 && terminals == 0 {
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
    if services > 0 {
        parts.push(pluralize(services, "service", "services"));
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
    let terminals = server_background_terminal_count(state);
    if prereqs == 0 && sidecars == 0 && services == 0 && terminals == 0 {
        None
    } else {
        Some(format!(
            "prereqs={prereqs} shell_sidecars={sidecars} services={services} terminals={terminals}"
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
    let mut lines = Vec::new();
    if matches!(filter, WorkerFilter::All | WorkerFilter::Blockers) {
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
        && !state.cached_agent_threads.is_empty()
    {
        push_section_gap(&mut lines);
        lines.extend(render_cached_agent_threads_section(
            &state.cached_agent_threads,
        ));
    }
    let shell_lines = match filter {
        WorkerFilter::All | WorkerFilter::Shells => state.background_shells.render_for_ps(),
        WorkerFilter::Services => state
            .background_shells
            .render_for_ps_filtered(Some(BackgroundShellIntent::Service)),
        WorkerFilter::Blockers => state
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

fn render_main_agent_section(state: &AppState, filter: WorkerFilter) -> Vec<String> {
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
    let dependencies = orchestration_dependency_edges(state)
        .into_iter()
        .filter(|edge| !matches!(filter, WorkerFilter::Blockers) || edge.blocking)
        .collect::<Vec<_>>();
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
    lines
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
        WorkerFilter::Agents => "No agent workers tracked right now.",
        WorkerFilter::Shells => "No local background shell jobs tracked right now.",
        WorkerFilter::Services => "No service shells tracked right now.",
        WorkerFilter::Terminals => "No server-observed background terminals tracked right now.",
    }
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
    use super::orchestration_overview_summary;
    use super::orchestration_prompt_suffix;
    use super::orchestration_runtime_summary;
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
        assert!(suffix.contains("1 service"));
        assert!(suffix.contains("1 terminal"));
        assert!(suffix.contains("/ps to view"));
        let background = orchestration_background_summary(&state).expect("background summary");
        assert!(background.contains("prereqs=1"));
        assert!(background.contains("shell_sidecars=0"));
        assert!(background.contains("services=1"));
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
                &serde_json::json!({"command": "sleep 0.4", "intent": "service", "label": "dev server"}),
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

        let services = render_orchestration_workers_with_filter(&state, WorkerFilter::Services);
        assert!(services.contains("Local background shell jobs:"));
        assert!(services.contains("intent   service"));
        assert!(services.contains("label    dev server"));
        assert!(!services.contains("intent   prerequisite"));

        let terminals = render_orchestration_workers_with_filter(&state, WorkerFilter::Terminals);
        assert!(terminals.contains("Server-observed background terminals:"));
        assert!(terminals.contains("python worker.py"));
        assert!(!terminals.contains("Local background shell jobs:"));
        let _ = state.background_shells.terminate_all_running();
    }
}
