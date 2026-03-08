#[path = "notification_item_completion.rs"]
mod notification_item_completion;
#[path = "notification_item_updates.rs"]
mod notification_item_updates;

use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

pub(crate) fn handle_turn_item_notification(
    notification: RpcNotification,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if notification.method == "item/completed" {
        notification_item_completion::render_item_completed(&notification.params, state, output)?;
        return Ok(true);
    }

    notification_item_updates::handle_update_notification(
        &notification.method,
        &notification.params,
        cli,
        state,
        output,
    )
}
