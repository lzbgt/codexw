use serde_json::Value;

use crate::orchestration_view::CachedAgentThreadSummary;
use crate::orchestration_view::summarize_agent_preview;
use crate::requests::ThreadListView;
use crate::state::get_string;
use crate::state::summarize_text;

fn sorted_threads(result: &Value) -> Vec<Value> {
    let mut threads = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    threads.sort_by(|left, right| {
        let left_updated = left
            .get("updatedAt")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        let right_updated = right
            .get("updatedAt")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MIN);
        right_updated.cmp(&left_updated).then_with(|| {
            get_string(left, &["id"])
                .unwrap_or("")
                .cmp(get_string(right, &["id"]).unwrap_or(""))
        })
    });
    threads
}

pub(crate) fn thread_list_is_empty(result: &Value) -> bool {
    sorted_threads(result).is_empty()
}

pub(crate) fn should_fallback_to_all_workspaces(
    result: &Value,
    search_term: Option<&str>,
    cwd_filter: Option<&str>,
) -> bool {
    cwd_filter.is_some() && search_term.is_none() && thread_list_is_empty(result)
}

pub(crate) fn render_thread_list(
    result: &Value,
    search_term: Option<&str>,
    view: ThreadListView,
) -> String {
    let threads = sorted_threads(result);
    if threads.is_empty() {
        return match (view, search_term) {
            (ThreadListView::Agents, _) => "No agents available yet.".to_string(),
            (ThreadListView::Threads, Some(search_term)) => {
                format!("No threads matched \"{search_term}\".")
            }
            (ThreadListView::Threads, None) => {
                "No threads found for the current workspace.".to_string()
            }
        };
    }
    let mut lines = Vec::new();
    if let Some(search_term) = search_term {
        lines.push(format!("Search: {search_term}"));
    }
    lines.extend(threads.iter().enumerate().map(|(index, thread)| {
        let id = get_string(thread, &["id"]).unwrap_or("?");
        let preview = get_string(thread, &["preview"]).unwrap_or("-");
        let status = get_string(thread, &["status", "type"]).unwrap_or("unknown");
        let updated_at = thread
            .get("updatedAt")
            .and_then(Value::as_i64)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "{:>2}. {id}  [{status}]  [updated {updated_at}]  {}",
            index + 1,
            summarize_text(preview)
        )
    }));
    let footer = match view {
        ThreadListView::Threads => "Use /resume <n> to resume one of these threads.",
        ThreadListView::Agents => "Use /resume <n> to switch to one of these agent threads.",
    };
    lines.push(footer.to_string());
    lines.join("\n")
}

pub(crate) fn extract_thread_ids(result: &Value) -> Vec<String> {
    sorted_threads(result)
        .iter()
        .filter_map(|thread| get_string(thread, &["id"]).map(ToOwned::to_owned))
        .collect()
}

pub(crate) fn extract_agent_thread_summaries(result: &Value) -> Vec<CachedAgentThreadSummary> {
    sorted_threads(result)
        .iter()
        .filter_map(|thread| {
            let id = get_string(thread, &["id"])?;
            Some(CachedAgentThreadSummary {
                id: id.to_string(),
                status: get_string(thread, &["status", "type"])
                    .unwrap_or("unknown")
                    .to_string(),
                preview: summarize_agent_preview(get_string(thread, &["preview"]).unwrap_or("-")),
                updated_at: thread.get("updatedAt").and_then(Value::as_i64),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::extract_agent_thread_summaries;
    use serde_json::json;

    #[test]
    fn agent_thread_summaries_keep_status_preview_and_sort_order() {
        let summaries = extract_agent_thread_summaries(&json!({
            "data": [
                {
                    "id": "agent-2",
                    "preview": "older item",
                    "updatedAt": 10,
                    "status": {"type": "idle"}
                },
                {
                    "id": "agent-1",
                    "preview": "latest item",
                    "updatedAt": 20,
                    "status": {"type": "active"}
                }
            ]
        }));

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, "agent-1");
        assert_eq!(summaries[0].status, "active");
        assert_eq!(summaries[0].preview, "latest item");
        assert_eq!(summaries[0].updated_at, Some(20));
        assert_eq!(summaries[1].id, "agent-2");
    }
}
