use serde_json::Value;
use serde_json::json;

use crate::Cli;

pub(crate) fn approval_policy(cli: &Cli) -> &'static str {
    let _ = cli;
    "never"
}

pub(crate) fn thread_sandbox_mode(cli: &Cli) -> &'static str {
    let _ = cli;
    "danger-full-access"
}

pub(crate) fn turn_sandbox_policy(cli: &Cli) -> Value {
    let _ = cli;
    json!({"type": "dangerFullAccess"})
}

pub(crate) fn reasoning_summary(cli: &Cli) -> &'static str {
    if cli.verbose_thinking {
        "detailed"
    } else {
        "auto"
    }
}

pub(crate) fn choose_command_approval_decision(params: &Value, yolo: bool) -> Value {
    let _ = yolo;
    if let Some(decisions) = params.get("availableDecisions").and_then(Value::as_array) {
        return choose_first_allowed_decision(decisions).unwrap_or_else(|| decisions[0].clone());
    }
    json!("accept")
}

pub(crate) fn choose_first_allowed_decision(decisions: &[Value]) -> Option<Value> {
    for preferred in [
        "acceptForSession",
        "acceptWithExecpolicyAmendment",
        "applyNetworkPolicyAmendment",
        "accept",
    ] {
        if let Some(found) = decisions
            .iter()
            .find(|decision| decision.as_str() == Some(preferred))
        {
            return Some(found.clone());
        }
    }
    None
}

pub(crate) fn shell_program() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}
