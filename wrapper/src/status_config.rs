use serde_json::Value;
use serde_json::json;

use crate::state::get_string;
use crate::status_value::summarize_value;

pub(crate) fn render_config_snapshot(result: &Value) -> String {
    if result.is_null() {
        return "config unavailable".to_string();
    }
    serde_json::to_string_pretty(result).unwrap_or_else(|_| summarize_value(result))
}

pub(crate) fn summarize_sandbox_policy(policy: &Value) -> String {
    match get_string(policy, &["type"]).unwrap_or("unknown") {
        "dangerFullAccess" => "dangerFullAccess".to_string(),
        other => summarize_value(&json!({
            "type": other,
            "policy": policy,
        })),
    }
}
