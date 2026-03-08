use serde_json::Value;

use crate::state::get_string;

pub(crate) fn summarize_thread_status_for_display(params: &Value) -> Option<String> {
    let status_type = get_string(params, &["status", "type"]).unwrap_or("unknown");
    let flags = params
        .get("status")
        .and_then(|v| v.get("activeFlags"))
        .and_then(Value::as_array)
        .map(|flags| flags.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    if status_type == "active" && flags.is_empty() {
        return None;
    }

    if flags.contains(&"waitingOnApproval") {
        return Some("waiting on approval".to_string());
    }

    if flags.is_empty() {
        if status_type == "idle" {
            Some("ready".to_string())
        } else {
            Some(status_type.to_string())
        }
    } else {
        Some(flags.join(", "))
    }
}

pub(crate) fn summarize_model_reroute(params: &Value) -> String {
    let from_model = get_string(params, &["fromModel"]).unwrap_or("?");
    let to_model = get_string(params, &["toModel"]).unwrap_or("?");
    let reason = get_string(params, &["reason"]).unwrap_or("unspecified");
    format!("{from_model} -> {to_model} reason={reason}")
}
