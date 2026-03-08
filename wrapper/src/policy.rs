use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::state::AppState;

pub(crate) fn approval_policy(cli: &Cli, state: &AppState) -> String {
    let _ = cli;
    state
        .session_overrides
        .approval_policy
        .clone()
        .unwrap_or_else(|| "never".to_string())
}

pub(crate) fn thread_sandbox_mode(cli: &Cli, state: &AppState) -> String {
    let _ = cli;
    state
        .session_overrides
        .thread_sandbox_mode
        .clone()
        .unwrap_or_else(|| "danger-full-access".to_string())
}

pub(crate) fn turn_sandbox_policy(cli: &Cli, state: &AppState) -> Value {
    let sandbox_mode = thread_sandbox_mode(cli, state);
    match sandbox_mode.as_str() {
        "read-only" => json!({
            "type": "readOnly",
            "networkAccess": false,
            "access": {"type": "fullAccess"},
        }),
        "workspace-write" => json!({
            "type": "workspaceWrite",
            "networkAccess": false,
        }),
        _ => json!({"type": "dangerFullAccess"}),
    }
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
