use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::background_terminals::clear_completed_command_item;
use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::transcript_completion_render::render_command_completion;
use crate::transcript_completion_render::render_file_change_completion;
use crate::transcript_item_summary::humanize_item_type;
use crate::transcript_item_summary::summarize_file_change_paths;
use crate::transcript_item_summary::summarize_tool_item;
use crate::transcript_plan_render::render_reasoning_item;

pub(crate) fn render_item_completed(
    cli: &Cli,
    params: &Value,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    let Some(item) = params.get("item") else {
        return Ok(());
    };
    let item_type = get_string(item, &["type"]).unwrap_or("unknown");
    clear_completed_item_status(item_type, item, cli, state);
    match item_type {
        "agentMessage" => {
            let text = get_string(item, &["text"]).unwrap_or("").to_string();
            state.last_agent_message = Some(text.clone());
            output.finish_stream()?;
            if !text.trim().is_empty() {
                output.block_stdout("Assistant", &text)?;
            }
        }
        "commandExecution" => {
            clear_completed_command_item(state, item);
            let status = get_string(item, &["status"]).unwrap_or("unknown");
            let command = get_string(item, &["command"]).unwrap_or("");
            let exit_code = item
                .get("exitCode")
                .and_then(Value::as_i64)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string());
            output.finish_stream()?;
            let item_id = get_string(item, &["id"]).unwrap_or("");
            let full_output = state
                .command_output_buffers
                .remove(item_id)
                .filter(|text| !text.trim().is_empty())
                .or_else(|| {
                    get_string(item, &["aggregatedOutput"])
                        .map(ToOwned::to_owned)
                        .filter(|text| !text.trim().is_empty())
                });
            let rendered = render_command_completion(
                command,
                status,
                &exit_code,
                full_output.as_deref(),
                cli.verbose_events || cli.raw_json,
            );
            output.block_stdout("Command complete", &rendered)?;
        }
        "fileChange" => {
            let status = get_string(item, &["status"]).unwrap_or("unknown");
            output.finish_stream()?;
            let item_id = get_string(item, &["id"]).unwrap_or("");
            let delta_output = state
                .file_output_buffers
                .remove(item_id)
                .filter(|text| !text.trim().is_empty());
            let rendered = render_file_change_completion(
                item,
                status,
                delta_output.as_deref(),
                cli.verbose_events || cli.raw_json,
            );
            output.block_stdout("File changes complete", &rendered)?;
        }
        "reasoning" => {
            output.finish_stream()?;
            let rendered = render_reasoning_item(item);
            if !rendered.is_empty() {
                output.block_stdout("Thinking", &rendered)?;
            }
        }
        "plan" => {
            output.finish_stream()?;
            let rendered = get_string(item, &["text"]).unwrap_or("").trim().to_string();
            if !rendered.is_empty() {
                output.block_stdout("Proposed Plan", &rendered)?;
            }
        }
        "mcpToolCall" | "dynamicToolCall" | "collabAgentToolCall" | "webSearch" => {
            output.finish_stream()?;
            output.block_stdout(
                &format!("{} complete", humanize_item_type(item_type)),
                &summarize_tool_item(item_type, item, cli.verbose_events || cli.raw_json),
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn clear_completed_item_status(item_type: &str, item: &Value, cli: &Cli, state: &mut AppState) {
    let expected = match item_type {
        "commandExecution" => get_string(item, &["command"])
            .map(|command| format!("running {}", summarize_text(command))),
        "fileChange" => Some(summarize_file_change_paths(item)),
        "mcpToolCall" | "dynamicToolCall" | "collabAgentToolCall" | "webSearch" | "plan" => {
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
