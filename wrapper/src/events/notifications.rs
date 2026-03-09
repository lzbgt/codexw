use anyhow::Result;
use serde_json::Value;
use std::process::ChildStdin;

use super::notification_realtime;
use crate::Cli;
use crate::config_persistence::persist_windows_sandbox_mode;
use crate::notification_item_buffers::handle_buffer_update;
use crate::notification_item_completion::render_item_completed;
use crate::notification_item_status::handle_status_update;
use crate::notification_turn_completed::handle_turn_completed;
use crate::notification_turn_started::handle_turn_started;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

pub(crate) fn handle_notification(
    notification: RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    if notification_realtime::handle_realtime_notification(&notification, cli, state, output)? {
        return Ok(());
    }

    match notification.method.as_str() {
        "skills/changed" => {
            crate::requests::send_load_skills(writer, state, resolved_cwd)?;
            return Ok(());
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
            return Ok(());
        }
        "turn/started" => {
            handle_turn_started(&notification, state);
            return Ok(());
        }
        "turn/completed" => {
            handle_turn_completed(&notification, cli, resolved_cwd, state, output, writer)?;
            return Ok(());
        }
        "item/completed" => {
            render_item_completed(cli, &notification.params, state, output)?;
            return Ok(());
        }
        _ => {}
    }

    if handle_buffer_update(
        &notification.method,
        &notification.params,
        cli,
        state,
        output,
    )? || handle_status_update(
        &notification.method,
        &notification.params,
        cli,
        state,
        output,
    )? {
        return Ok(());
    }
    Ok(())
}
