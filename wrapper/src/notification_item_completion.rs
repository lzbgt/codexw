mod item;
mod status;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::notification_item_completion::item::render_completed_item;
use crate::notification_item_completion::status::clear_completed_item_status;
use crate::notification_item_completion::status::reconcile_collab_wait_status_line;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn render_item_completed(
    cli: &Cli,
    params: &Value,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    let Some(item) = params.get("item") else {
        return Ok(());
    };
    let item_type = crate::state::get_string(item, &["type"]).unwrap_or("unknown");
    if item_type == "collabAgentToolCall" {
        crate::orchestration_registry::track_collab_agent_task_completed(state, item);
        reconcile_collab_wait_status_line(state);
    }
    clear_completed_item_status(item_type, item, cli, state);
    render_completed_item(item_type, item, cli, state, output)
}
