use serde_json::Value;

use crate::Cli;
use crate::orchestration_registry::wait_dependency_summary;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::transcript_item_summary::humanize_item_type;
use crate::transcript_item_summary::summarize_file_change_paths;
use crate::transcript_item_summary::summarize_tool_item;

pub(crate) fn clear_completed_item_status(
    item_type: &str,
    item: &Value,
    cli: &Cli,
    state: &mut AppState,
) {
    let expected = match item_type {
        "commandExecution" => get_string(item, &["command"])
            .map(|command| format!("running {}", summarize_text(command))),
        "fileChange" => Some(summarize_file_change_paths(item)),
        "mcpToolCall" | "collabAgentToolCall" | "webSearch" | "plan" => {
            Some(summarize_text(&format!(
                "{} {}",
                humanize_item_type(item_type),
                summarize_tool_item(item_type, item, cli.verbose_events || cli.raw_json)
            )))
        }
        _ => None,
    };

    if expected.as_deref() == state.last_status_line.as_deref() {
        state.last_status_line = None;
    }
}

pub(crate) fn reconcile_collab_wait_status_line(state: &mut AppState) {
    if matches!(
        state.last_status_line.as_deref(),
        Some(line) if line.starts_with("waiting on agent")
    ) {
        state.last_status_line = wait_dependency_summary(state);
    }
}
