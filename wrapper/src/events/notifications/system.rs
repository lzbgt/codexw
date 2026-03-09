use anyhow::Result;
use serde_json::Value;
use std::process::ChildStdin;

use crate::config_persistence::persist_windows_sandbox_mode;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

pub(crate) fn handle_system_notification(
    notification: &RpcNotification,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match notification.method.as_str() {
        "skills/changed" => {
            crate::requests::send_load_skills(writer, state, resolved_cwd)?;
            Ok(true)
        }
        "windowsSandbox/setupCompleted" => {
            let mode = notification
                .params
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let success = notification
                .params
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let error = notification.params.get("error").and_then(Value::as_str);
            if success {
                persist_windows_sandbox_mode(state.codex_home_override.as_deref(), Some(mode))?;
                output.line_stderr(format!(
                    "[session] Windows sandbox setup completed successfully ({mode})"
                ))?;
            } else {
                let detail = error.unwrap_or("unknown error");
                output.line_stderr(format!(
                    "[session] Windows sandbox setup failed ({mode}): {detail}"
                ))?;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}
