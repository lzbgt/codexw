use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

#[path = "notification_realtime.rs"]
mod notification_realtime;
#[path = "notification_turns.rs"]
mod notification_turns;

use notification_realtime::handle_realtime_notification;
use notification_turns::handle_turn_notification;

pub(crate) fn handle_notification(
    notification: RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    if handle_realtime_notification(&notification, cli, state, output)? {
        return Ok(());
    }
    handle_turn_notification(notification, cli, resolved_cwd, state, output, writer)
}
