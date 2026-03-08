use std::time::Instant;

use anyhow::Result;

use crate::Cli;
use crate::catalog::parse_apps_list;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::session_realtime::render_realtime_item;
use crate::state::AppState;
use crate::state::emit_status_line;
use crate::state::get_string;
use crate::transcript_summary::summarize_thread_status_for_display;

pub(crate) fn handle_realtime_notification(
    notification: &RpcNotification,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match notification.method.as_str() {
        "thread/started" => {
            if let Some(thread_id) = get_string(&notification.params, &["thread", "id"]) {
                state.thread_id = Some(thread_id.to_string());
            }
        }
        "app/list/updated" => {
            state.apps = parse_apps_list(&notification.params);
        }
        "account/updated" => {
            state.account_info = notification.params.get("account").cloned().or_else(|| {
                let auth_mode = notification.params.get("authMode")?.clone();
                let plan_type = notification
                    .params
                    .get("planType")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                Some(serde_json::json!({
                    "type": auth_mode,
                    "planType": plan_type,
                }))
            });
        }
        "account/rateLimits/updated" => {
            state.rate_limits = notification.params.get("rateLimits").cloned();
        }
        "thread/status/changed" => {
            if let Some(status_line) = summarize_thread_status_for_display(&notification.params) {
                emit_status_line(output, state, status_line)?;
            }
        }
        "thread/tokenUsage/updated" => {
            state.last_token_usage = notification.params.get("tokenUsage").cloned();
        }
        "thread/realtime/started" => {
            state.realtime_active = true;
            state.realtime_started_at = Some(Instant::now());
            state.realtime_session_id =
                get_string(&notification.params, &["sessionId"]).map(ToOwned::to_owned);
            state.realtime_last_error = None;
            let session = state.realtime_session_id.as_deref().unwrap_or("ephemeral");
            output.line_stderr(format!("[realtime] active session={session}"))?;
        }
        "thread/realtime/itemAdded" => {
            if let Some(item) = notification.params.get("item") {
                output.block_stdout("Realtime", &render_realtime_item(item))?;
            }
        }
        "thread/realtime/outputAudio/delta" => {
            if cli.verbose_events {
                output.line_stderr("[realtime] received output audio chunk (not rendered)")?;
            }
        }
        "thread/realtime/error" => {
            let message = get_string(&notification.params, &["message"])
                .unwrap_or("unknown realtime error")
                .to_string();
            state.realtime_last_error = Some(message.clone());
            output.line_stderr(format!("[realtime-error] {message}"))?;
        }
        "thread/realtime/closed" => {
            let reason = get_string(&notification.params, &["reason"]).map(ToOwned::to_owned);
            state.realtime_active = false;
            state.realtime_session_id = None;
            state.realtime_started_at = None;
            if let Some(reason) = reason {
                output.line_stderr(format!("[realtime] closed: {reason}"))?;
            } else {
                output.line_stderr("[realtime] closed")?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
