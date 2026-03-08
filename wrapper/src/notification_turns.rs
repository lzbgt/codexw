#[path = "notification_turn_items.rs"]
mod notification_turn_items;
#[path = "notification_turn_lifecycle.rs"]
mod notification_turn_lifecycle;

use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

pub(crate) fn handle_turn_notification(
    notification: RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
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
