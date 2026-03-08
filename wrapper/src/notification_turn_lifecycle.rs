use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

#[path = "notification_turn_completed.rs"]
mod notification_turn_completed;
#[path = "notification_turn_started.rs"]
mod notification_turn_started;

pub(crate) fn handle_turn_lifecycle_notification(
    notification: &RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match notification.method.as_str() {
        "skills/changed" => {
            crate::requests::send_load_skills(writer, state, resolved_cwd)?;
        }
        "turn/started" => {
            notification_turn_started::handle_turn_started(notification, state);
        }
        "turn/completed" => {
            notification_turn_completed::handle_turn_completed(
                notification,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
            )?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
