use serde_json::Value;

use crate::orchestration_view::CachedAgentThreadSummary;
use crate::orchestration_view::summarize_agent_preview;
use crate::requests::ThreadListView;
use crate::state::get_string;
use crate::state::summarize_text;

#[derive(Debug, Clone)]
pub(crate) struct ThreadListEntry {
    pub(crate) id: String,
    pub(crate) preview: String,
    pub(crate) status: String,
    pub(crate) updated_at: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ThreadListSnapshot {
    entries: Vec<ThreadListEntry>,
}

pub(crate) fn thread_list_snapshot(result: &Value) -> ThreadListSnapshot {
    let mut entries = result
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|thread| {
            let id = get_string(thread, &["id"])?;
            Some(ThreadListEntry {
                id: id.to_string(),
                preview: get_string(thread, &["preview"]).unwrap_or("-").to_string(),
                status: get_string(thread, &["status", "type"])
                    .unwrap_or("unknown")
                    .to_string(),
                updated_at: thread.get("updatedAt").and_then(Value::as_i64),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .updated_at
            .unwrap_or(i64::MIN)
            .cmp(&left.updated_at.unwrap_or(i64::MIN))
            .then_with(|| left.id.cmp(&right.id))
    });
    ThreadListSnapshot { entries }
}

impl ThreadListSnapshot {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn thread_ids(&self) -> Vec<String> {
        self.entries.iter().map(|entry| entry.id.clone()).collect()
    }

    pub(crate) fn agent_thread_summaries(&self) -> Vec<CachedAgentThreadSummary> {
        self.entries
            .iter()
            .map(|thread| CachedAgentThreadSummary {
                id: thread.id.clone(),
                status: thread.status.clone(),
                preview: summarize_agent_preview(&thread.preview),
                updated_at: thread.updated_at,
            })
            .collect()
    }

    pub(crate) fn render(&self, search_term: Option<&str>, view: ThreadListView) -> String {
        if self.entries.is_empty() {
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
        lines.extend(self.entries.iter().enumerate().map(|(index, thread)| {
            let updated_at = thread
                .updated_at
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            format!(
                "{:>2}. {}  [{}]  [updated {}]  {}",
                index + 1,
                thread.id,
                thread.status,
                updated_at,
                summarize_text(&thread.preview)
            )
        }));
        let footer = match view {
            ThreadListView::Threads => "Use /resume <n> to resume one of these threads.",
            ThreadListView::Agents => "Use /resume <n> to switch to one of these agent threads.",
        };
        lines.push(footer.to_string());
        lines.join("\n")
    }
}

pub(crate) fn thread_list_is_empty(result: &Value) -> bool {
    thread_list_snapshot(result).is_empty()
}

pub(crate) fn should_fallback_to_all_workspaces(
    result: &Value,
    search_term: Option<&str>,
    cwd_filter: Option<&str>,
) -> bool {
    cwd_filter.is_some() && search_term.is_none() && thread_list_is_empty(result)
}

#[cfg(test)]
pub(crate) fn render_thread_list(
    result: &Value,
    search_term: Option<&str>,
    view: ThreadListView,
) -> String {
    thread_list_snapshot(result).render(search_term, view)
}

#[cfg(test)]
pub(crate) fn extract_thread_ids(result: &Value) -> Vec<String> {
    thread_list_snapshot(result).thread_ids()
}

#[cfg(test)]
pub(crate) fn extract_agent_thread_summaries(result: &Value) -> Vec<CachedAgentThreadSummary> {
    thread_list_snapshot(result).agent_thread_summaries()
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
