use serde_json::Value;

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

pub(crate) fn render_thread_list(result: &Value, search_term: Option<&str>) -> String {
    let threads = sorted_threads(result);
    if threads.is_empty() {
        return match search_term {
            Some(search_term) => format!("No threads matched \"{search_term}\"."),
            None => "No threads found for the current workspace.".to_string(),
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
    lines.push("Use /resume <n> to resume one of these threads.".to_string());
    lines.join("\n")
}

pub(crate) fn extract_thread_ids(result: &Value) -> Vec<String> {
    sorted_threads(result)
        .iter()
        .filter_map(|thread| get_string(thread, &["id"]).map(ToOwned::to_owned))
        .collect()
}
