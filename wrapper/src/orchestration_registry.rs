use std::collections::BTreeMap;

use serde_json::Value;

use crate::orchestration_view::CachedAgentThreadSummary;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LiveAgentTaskSummary {
    pub(crate) id: String,
    pub(crate) tool: String,
    pub(crate) status: String,
    pub(crate) sender_thread_id: String,
    pub(crate) receiver_thread_ids: Vec<String>,
    pub(crate) prompt: Option<String>,
    pub(crate) agent_statuses: BTreeMap<String, String>,
}

pub(crate) fn track_collab_agent_task_started(state: &mut AppState, item: &Value) {
    let Some(task) = parse_live_agent_task(item) else {
        return;
    };
    merge_cached_agent_threads(state, &task, item);
    state.live_agent_tasks.insert(task.id.clone(), task);
}

pub(crate) fn track_collab_agent_task_completed(state: &mut AppState, item: &Value) {
    let Some(task) = parse_live_agent_task(item) else {
        return;
    };
    merge_cached_agent_threads(state, &task, item);
    state.live_agent_tasks.remove(&task.id);
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
            &mut state.cached_agent_threads,
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

#[cfg(test)]
mod tests {
    use super::track_collab_agent_task_completed;
    use super::track_collab_agent_task_started;
    use serde_json::json;

    #[test]
    fn collab_agent_task_tracking_updates_live_registry_and_cached_threads() {
        let mut state = crate::state::AppState::new(true, false);
        let item = json!({
            "type": "collabAgentToolCall",
            "id": "call-1",
            "tool": "spawnAgent",
            "status": "inProgress",
            "senderThreadId": "thread-main",
            "receiverThreadIds": ["thread-agent-1"],
            "prompt": "Inspect auth flow and report risks",
            "agentsStates": {
                "thread-agent-1": {
                    "status": "running",
                    "message": "reviewing auth flow"
                }
            }
        });

        track_collab_agent_task_started(&mut state, &item);

        assert_eq!(state.live_agent_tasks.len(), 1);
        assert_eq!(state.cached_agent_threads.len(), 1);
        assert_eq!(state.cached_agent_threads[0].id, "thread-agent-1");
        assert_eq!(state.cached_agent_threads[0].status, "running");
        assert_eq!(state.cached_agent_threads[0].preview, "reviewing auth flow");

        let completed = json!({
            "type": "collabAgentToolCall",
            "id": "call-1",
            "tool": "spawnAgent",
            "status": "completed",
            "senderThreadId": "thread-main",
            "receiverThreadIds": ["thread-agent-1"],
            "agentsStates": {
                "thread-agent-1": {
                    "status": "completed",
                    "message": "done"
                }
            }
        });
        track_collab_agent_task_completed(&mut state, &completed);

        assert!(state.live_agent_tasks.is_empty());
        assert_eq!(state.cached_agent_threads[0].status, "completed");
        assert_eq!(state.cached_agent_threads[0].preview, "done");
    }
}
