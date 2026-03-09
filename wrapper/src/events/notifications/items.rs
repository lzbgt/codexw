use anyhow::Result;

use crate::Cli;
use crate::notification_item_buffers::handle_buffer_update;
use crate::notification_item_completion::render_item_completed;
use crate::notification_item_status::handle_status_update;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;

pub(crate) fn handle_item_notification(
    notification: &RpcNotification,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if notification.method == "item/completed" {
        render_item_completed(cli, &notification.params, state, output)?;
        return Ok(true);
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
        return Ok(true);
    }

    Ok(false)
}
