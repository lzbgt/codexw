use anyhow::Result;
use std::process::ChildStdin;

use crate::Cli;
use crate::output::Output;
use crate::responses::handle_response;
use crate::rpc;
use crate::rpc::IncomingMessage;
use crate::rpc::RpcNotification;
use crate::runtime_process::StartMode;
use crate::state::AppState;

#[path = "event_requests.rs"]
mod event_requests;
#[path = "notification_realtime.rs"]
mod notification_realtime;
#[path = "notification_turn_items.rs"]
mod notification_turn_items;
#[path = "notification_turn_lifecycle.rs"]
mod notification_turn_lifecycle;

use event_requests::handle_server_request;
#[cfg(test)]
pub(crate) use event_requests::params_auto_approval_result;

pub(crate) fn process_server_line(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if state.raw_json {
        output.line_stderr(format!("[json] {line}"))?;
    }
    match rpc::parse_line(&line) {
        Ok(IncomingMessage::Response(response)) => handle_response(
            response,
            cli,
            resolved_cwd,
            state,
            output,
            writer,
            start_after_initialize,
        )?,
        Ok(IncomingMessage::Request(request)) => {
            handle_server_request(request, cli, output, writer)?;
        }
        Ok(IncomingMessage::Notification(notification)) => {
            handle_notification(notification, cli, resolved_cwd, state, output, writer)?;
        }
        Err(err) => {
            output.line_stderr(format!("[session] ignored malformed server line: {err}"))?;
        }
    }
    Ok(())
}

fn handle_notification(
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

    if notification_turn_lifecycle::handle_turn_lifecycle_notification(
        &notification,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
    )? {
        return Ok(());
    }

    notification_turn_items::handle_turn_item_notification(notification, cli, state, output)?;
    Ok(())
}
