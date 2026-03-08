pub(crate) use crate::status_account::render_account_summary;
pub(crate) use crate::status_config::render_config_snapshot;
pub(crate) use crate::status_config::render_permissions_snapshot;
pub(crate) use crate::status_config::summarize_sandbox_policy;
pub(crate) use crate::status_token_usage::render_token_usage_summary;
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

pub(crate) fn render_rate_limit_lines(rate_limits: Option<&Value>) -> Vec<String> {
    let Some(rate_limits) = rate_limits else {
        return vec!["rate limits     unavailable".to_string()];
    };

    let mut lines = crate::status_rate_windows::render_window_lines(rate_limits);
    let first_row = lines.is_empty();

    if let Some(credits) = rate_limits.get("credits")
        && let Some(credit_line) =
            crate::status_rate_credits::render_credit_line(credits, first_row)
    {
        lines.push(credit_line);
    }

    if lines.is_empty() {
        vec!["rate limits     none reported".to_string()]
    } else {
        lines
    }
}
