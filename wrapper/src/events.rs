use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;

use crate::Cli;
use crate::output::Output;
use crate::rpc;
use crate::rpc::IncomingMessage;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_process::StartMode;
use crate::state::AppState;

#[path = "notification_realtime.rs"]
mod notification_realtime;
#[path = "events/notifications.rs"]
mod notifications;
#[path = "events/requests.rs"]
mod requests;
#[path = "events/responses.rs"]
mod responses;

#[cfg(test)]
pub(crate) use crate::event_request_approvals::params_auto_approval_result;
#[cfg(test)]
pub(crate) use notification_realtime::handle_realtime_notification;

pub(crate) fn process_server_line(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    tx: &mpsc::Sender<AppEvent>,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if state.raw_json {
        output.line_stderr(format!("[json] {line}"))?;
    }
    match rpc::parse_line(&line) {
        Ok(IncomingMessage::Response(response)) => responses::handle_response(
            response,
            cli,
            resolved_cwd,
            state,
            output,
            writer,
            start_after_initialize,
        )?,
        Ok(IncomingMessage::Request(request)) => {
            requests::handle_server_request(request, cli, resolved_cwd, state, output, writer, tx)?;
        }
        Ok(IncomingMessage::Notification(notification)) => {
            notifications::handle_notification(
                notification,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
            )?;
        }
        Err(err) => {
            output.line_stderr(format!("[session] ignored malformed server line: {err}"))?;
        }
    }
    Ok(())
}
