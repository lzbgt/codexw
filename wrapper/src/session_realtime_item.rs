use serde_json::Value;

use crate::state::get_string;
use crate::status_views::summarize_value;

pub(crate) fn render_realtime_item(item: &Value) -> String {
    let item_type = get_string(item, &["type"]).unwrap_or("item");
    let item_id = get_string(item, &["id"]).unwrap_or("-");
    let role = get_string(item, &["role"]).unwrap_or("-");
    let body = extract_realtime_text(item).unwrap_or_else(|| summarize_value(item));
    format!(
        "type            {item_type}\nid              {item_id}\nrole            {role}\n\n{}",
        body.trim()
    )
}

fn extract_realtime_text(item: &Value) -> Option<String> {
    if let Some(text) = get_string(item, &["text"]).filter(|text| !text.trim().is_empty()) {
        return Some(text.to_string());
    }
    if let Some(text) = get_string(item, &["transcript"]).filter(|text| !text.trim().is_empty()) {
        return Some(text.to_string());
    }
    item.get("content")
        .and_then(Value::as_array)
        .and_then(|content| {
            let pieces = content
                .iter()
                .filter_map(|part| {
                    get_string(part, &["text"])
                        .or_else(|| get_string(part, &["transcript"]))
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>();
            if pieces.is_empty() {
                None
            } else {
                Some(pieces.join("\n\n"))
            }
        })
}
