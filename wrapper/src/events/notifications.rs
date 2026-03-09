#[path = "notifications/items.rs"]
mod items;
#[path = "notifications/system.rs"]
mod system;
#[path = "notifications/turns.rs"]
mod turns;

use anyhow::Result;
use std::process::ChildStdin;

use super::notification_realtime;
use crate::Cli;
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

    if system::handle_system_notification(&notification, resolved_cwd, state, output, writer)? {
        return Ok(());
    }
    if turns::handle_turn_notification(&notification, cli, resolved_cwd, state, output, writer)? {
        return Ok(());
    }
    if items::handle_item_notification(&notification, cli, state, output)? {
        return Ok(());
    }
    Ok(())
}
