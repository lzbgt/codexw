use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::background_terminals::track_started_command_item;
use crate::orchestration_registry::track_collab_agent_task_started;
use crate::orchestration_registry::wait_dependency_summary;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::transcript_item_summary::humanize_item_type;
use crate::transcript_item_summary::summarize_file_change_paths;
use crate::transcript_item_summary::summarize_tool_item;

pub(crate) fn render_item_started(params: &Value, cli: &Cli, state: &mut AppState) -> Result<()> {
    let Some(item) = params.get("item") else {
        return Ok(());
    };
    let item_type = get_string(item, &["type"]).unwrap_or("unknown");
    match item_type {
        "commandExecution" => {
            track_started_command_item(state, item);
            let command = get_string(item, &["command"]).unwrap_or("");
            state.last_status_line = Some(format!("running {}", summarize_text(command)));
        }
        "fileChange" => {
            state.last_status_line = Some(summarize_file_change_paths(item));
        }
        "agentMessage" | "reasoning" => {}
        "mcpToolCall" | "dynamicToolCall" | "webSearch" | "plan" => {
            state.last_status_line = Some(summarize_text(&format!(
                "{} {}",
                humanize_item_type(item_type),
                summarize_tool_item(item_type, item, cli.verbose_events || cli.raw_json)
            )));
        }
        "collabAgentToolCall" => {
            track_collab_agent_task_started(state, item);
            state.last_status_line = wait_dependency_summary(state).or_else(|| {
                Some(summarize_text(&format!(
                    "{} {}",
                    humanize_item_type(item_type),
                    summarize_tool_item(item_type, item, cli.verbose_events || cli.raw_json)
                )))
            });
        }
        _ => {}
    }
    Ok(())
}
