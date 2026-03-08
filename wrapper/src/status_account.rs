use serde_json::Value;

use crate::state::get_string;

pub(crate) fn render_account_summary(account: Option<&Value>) -> Option<String> {
    let account = account?;
    if account.is_null() {
        return Some("not signed in".to_string());
    }
    let account_type = get_string(account, &["type"])
        .or_else(|| get_string(account, &["authMode"]))
        .unwrap_or("unknown");
    let mut parts = vec![account_type.to_string()];
    if let Some(email) = get_string(account, &["email"]) {
        parts.push(email.to_string());
    }
    if let Some(plan_type) = get_string(account, &["planType"]) {
        parts.push(format!("plan={plan_type}"));
    }
    Some(parts.join(" "))
}
