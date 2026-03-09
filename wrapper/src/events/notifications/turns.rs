use anyhow::Result;
use std::process::ChildStdin;

use crate::Cli;
use crate::notification_turn_completed::handle_turn_completed;
use crate::notification_turn_started::handle_turn_started;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

pub(crate) fn handle_turn_notification(
    notification: &RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match notification.method.as_str() {
        "turn/started" => {
            handle_turn_started(notification, state);
            Ok(true)
        }
        "turn/completed" => {
            handle_turn_completed(notification, cli, resolved_cwd, state, output, writer)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
