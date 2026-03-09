use std::collections::BTreeMap;

use serde_json::Value;

use crate::orchestration_view::CachedAgentThreadSummary;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;

use super::LiveAgentTaskSummary;

pub(crate) fn track_collab_agent_task_started(state: &mut AppState, item: &Value) {
    let Some(task) = parse_live_agent_task(item) else {
        return;
    };
    merge_cached_agent_threads(state, &task, item);
    state
        .orchestration
        .live_agent_tasks
        .insert(task.id.clone(), task);
}

pub(crate) fn track_collab_agent_task_completed(state: &mut AppState, item: &Value) {
    let Some(task) = parse_live_agent_task(item) else {
        return;
    };
    merge_cached_agent_threads(state, &task, item);
    state.orchestration.live_agent_tasks.remove(&task.id);
}

fn parse_live_agent_task(item: &Value) -> Option<LiveAgentTaskSummary> {
    if get_string(item, &["type"]) != Some("collabAgentToolCall") {
        return None;
    }
    let id = get_string(item, &["id"])?;
    let tool = get_string(item, &["tool"]).unwrap_or("unknown");
    let status = get_string(item, &["status"]).unwrap_or("unknown");
    let sender_thread_id = get_string(item, &["senderThreadId"]).unwrap_or("-");
    let receiver_thread_ids = item
        .get("receiverThreadIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let prompt = get_string(item, &["prompt"])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(summarize_text);
    let agent_statuses = parse_agent_statuses(item.get("agentsStates"));
    Some(LiveAgentTaskSummary {
        id: id.to_string(),
        tool: tool.to_string(),
        status: status.to_string(),
        sender_thread_id: sender_thread_id.to_string(),
        receiver_thread_ids,
        prompt,
        agent_statuses,
    })
}

fn parse_agent_statuses(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|states| {
            states
                .iter()
                .map(|(thread_id, state)| {
                    let status = get_string(state, &["status"]).unwrap_or("unknown");
                    (thread_id.clone(), status.to_string())
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn merge_cached_agent_threads(state: &mut AppState, task: &LiveAgentTaskSummary, item: &Value) {
    for thread_id in &task.receiver_thread_ids {
        let status = task
            .agent_statuses
            .get(thread_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let preview = item
            .get("agentsStates")
            .and_then(|states| states.get(thread_id))
            .and_then(|agent| get_string(agent, &["message"]))
            .map(summarize_text)
            .or_else(|| task.prompt.clone())
            .unwrap_or_else(|| "-".to_string());
        upsert_cached_agent_thread(
            &mut state.orchestration.cached_agent_threads,
            CachedAgentThreadSummary {
                id: thread_id.clone(),
                status,
                preview,
                updated_at: None,
            },
        );
    }
}

fn upsert_cached_agent_thread(
    cached: &mut Vec<CachedAgentThreadSummary>,
    incoming: CachedAgentThreadSummary,
) {
    if let Some(existing) = cached.iter_mut().find(|thread| thread.id == incoming.id) {
        existing.status = incoming.status;
        if incoming.preview != "-" {
            existing.preview = incoming.preview;
        }
        if incoming.updated_at.is_some() {
            existing.updated_at = incoming.updated_at;
        }
        return;
    }
    cached.push(incoming);
    cached.sort_by(|left, right| left.id.cmp(&right.id));
}
