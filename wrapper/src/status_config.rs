use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::policy::turn_sandbox_policy;
use crate::state::get_string;
use crate::status_views::summarize_value;

pub(crate) fn render_permissions_snapshot(cli: &Cli) -> String {
    [
        format!("approval policy  {}", approval_policy(cli)),
        format!("thread sandbox   {}", thread_sandbox_mode(cli)),
        format!(
            "turn sandbox     {}",
            summarize_sandbox_policy(&turn_sandbox_policy(cli))
        ),
        "network access    enabled".to_string(),
        "tool use          automatic".to_string(),
        "shell exec        automatic".to_string(),
        "host access       full".to_string(),
    ]
    .join("\n")
}

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
