use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;

#[path = "notification_item_buffers.rs"]
mod notification_item_buffers;
#[path = "notification_item_status.rs"]
mod notification_item_status;

pub(crate) fn handle_update_notification(
    method: &str,
    params: &Value,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if notification_item_buffers::handle_buffer_update(method, params, cli, state, output)? {
        return Ok(true);
    }
    if notification_item_status::handle_status_update(method, params, cli, state, output)? {
        return Ok(true);
    }
    Ok(false)
}
