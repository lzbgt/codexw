use serde_json::Value;

use crate::state::get_string;
use crate::state::summarize_text;

pub(crate) fn render_thread_list(result: &Value, search_term: Option<&str>) -> String {
    let threads = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
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
    result
        .get("data")
        .and_then(Value::as_array)
        .map(|threads| {
            threads
                .iter()
                .filter_map(|thread| get_string(thread, &["id"]).map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
