pub(crate) use crate::status_account::render_account_summary;
pub(crate) use crate::status_config::render_config_snapshot;
pub(crate) use crate::status_config::render_permissions_snapshot;
pub(crate) use crate::status_config::summarize_sandbox_policy;
pub(crate) use crate::status_limits::render_rate_limit_lines;
pub(crate) use crate::status_limits::render_token_usage_summary;
use serde_json::Value;

pub(crate) fn summarize_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => string.to_string(),
        Value::Array(array) => {
            if array.is_empty() {
                "[]".to_string()
            } else {
                format!("[{} items]", array.len())
            }
        }
        Value::Object(object) => object
            .iter()
            .take(6)
            .map(|(key, value)| format!("{key}={}", summarize_value(value)))
            .collect::<Vec<_>>()
            .join(" "),
    }
}
