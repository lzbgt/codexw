use serde_json::Value;

pub(crate) fn render_credit_line(credits: &Value, first_row: bool) -> Option<String> {
    if !credits
        .get("hasCredits")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let prefix = if first_row {
        "rate limits     "
    } else {
        "                "
    };
    if credits
        .get("unlimited")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Some(format!("{prefix}credits unlimited"));
    }
    let balance = credits.get("balance").and_then(Value::as_str)?.trim();
    if balance.is_empty() {
        return None;
    }
    Some(format!("{prefix}credits {balance}"))
}
