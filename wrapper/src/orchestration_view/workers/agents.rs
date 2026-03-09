use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_registry::task_role;
use crate::state::AppState;

use super::super::CachedAgentThreadSummary;

pub(crate) fn live_agent_tasks(state: &AppState) -> Vec<LiveAgentTaskSummary> {
    let mut tasks = state
        .orchestration
        .live_agent_tasks
        .values()
        .cloned()
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    tasks
}

pub(super) fn render_live_agent_tasks_section(tasks: &[LiveAgentTaskSummary]) -> Vec<String> {
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

pub(super) fn render_cached_agent_threads_section(
    agent_threads: &[CachedAgentThreadSummary],
) -> Vec<String> {
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
