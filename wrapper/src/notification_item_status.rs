use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::background_terminals::track_started_command_item;
use crate::orchestration_registry::track_collab_agent_task_started;
use crate::orchestration_registry::wait_dependency_summary;
use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::transcript_approval_summary::summarize_server_request_resolved;
use crate::transcript_item_summary::humanize_item_type;
use crate::transcript_item_summary::summarize_file_change_paths;
use crate::transcript_item_summary::summarize_tool_item;
use crate::transcript_status_summary::summarize_model_reroute;

pub(crate) fn handle_status_update(
    method: &str,
    params: &Value,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match method {
        "model/rerouted" => {
            output.line_stderr(format!("[model] {}", summarize_model_reroute(params)))?;
        }
        "item/started" => render_item_started(params, cli, state)?,
        "item/agentMessage/delta"
        | "item/reasoning/summaryTextDelta"
        | "item/reasoning/textDelta"
        | "item/reasoning/summaryPartAdded" => {}
        "serverRequest/resolved" => {
            if state.last_status_line.as_deref() == Some("waiting on approval") {
                state.last_status_line = None;
            }
            if cli.verbose_events {
                output.line_stderr(format!(
                    "[approval] resolved {}",
                    summarize_server_request_resolved(params)
                ))?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn render_item_started(params: &Value, cli: &Cli, state: &mut AppState) -> Result<()> {
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
