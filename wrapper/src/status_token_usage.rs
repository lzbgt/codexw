use serde_json::Value;

pub(crate) fn render_token_usage_summary(token_usage: Option<&Value>) -> Option<String> {
    let token_usage = token_usage?;
    let last_total = token_usage
        .get("last")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    let cumulative_total = token_usage
        .get("total")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    match (last_total, cumulative_total) {
        (Some(last_total), Some(cumulative_total)) => {
            Some(format!("last={} total={}", last_total, cumulative_total))
        }
        (Some(last_total), None) => Some(format!("last={last_total}")),
        (None, Some(cumulative_total)) => Some(format!("total={cumulative_total}")),
        (None, None) => None,
    }
}
