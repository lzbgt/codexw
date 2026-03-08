use serde_json::Value;

use crate::session_prompt_status::format_elapsed;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::views::summarize_value;

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    let mut lines = vec![format!("active          {}", state.realtime_active)];
    lines.push(format!(
        "session         {}",
        state.realtime_session_id.as_deref().unwrap_or("-")
    ));
    lines.push(format!(
        "prompt          {}",
        summarize_text(state.realtime_prompt.as_deref().unwrap_or("-"))
    ));
    if state.realtime_active {
        lines.push(format!(
            "active time     {}",
            format_elapsed(state.realtime_started_at)
        ));
    }
    if let Some(error) = state.realtime_last_error.as_deref() {
        lines.push(format!("last error      {}", summarize_text(error)));
    }
    lines.push(
        "commands        /realtime start [prompt...] | /realtime send <text> | /realtime stop"
            .to_string(),
    );
    lines.push("audio           output audio deltas are not rendered in codexw".to_string());
    lines.join("\n")
}

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
