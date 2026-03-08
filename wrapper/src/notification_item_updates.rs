use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;
use crate::state::buffer_item_delta;
use crate::state::buffer_process_delta;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::status_views::summarize_value;
use crate::transcript_render::format_plan;
use crate::transcript_summary::humanize_item_type;
use crate::transcript_summary::summarize_file_change_paths;
use crate::transcript_summary::summarize_model_reroute;
use crate::transcript_summary::summarize_server_request_resolved;
use crate::transcript_summary::summarize_terminal_interaction;
use crate::transcript_summary::summarize_tool_item;

pub(crate) fn handle_update_notification(
    method: &str,
    params: &Value,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match method {
        "command/exec/outputDelta" => {
            buffer_process_delta(&mut state.process_output_buffers, params);
        }
        "turn/diff/updated" => {
            state.last_turn_diff = get_string(params, &["diff"]).map(ToOwned::to_owned);
            if cli.verbose_events
                && let Some(diff) = get_string(params, &["diff"])
            {
                output.line_stdout("[diff]")?;
                output.line_stdout(diff)?;
            }
        }
        "turn/plan/updated" => {
            let plan_text = format_plan(params);
            if !plan_text.is_empty() {
                output.line_stdout("[plan]")?;
                output.line_stdout(plan_text)?;
            }
        }
        "model/rerouted" => {
            output.line_stderr(format!("[model] {}", summarize_model_reroute(params)))?;
        }
        "item/started" => render_item_started(params, state)?,
        "item/agentMessage/delta"
        | "item/reasoning/summaryTextDelta"
        | "item/reasoning/textDelta"
        | "item/reasoning/summaryPartAdded" => {}
        "item/commandExecution/outputDelta" => {
            buffer_item_delta(&mut state.command_output_buffers, params)
        }
        "item/fileChange/outputDelta" => buffer_item_delta(&mut state.file_output_buffers, params),
        "item/commandExecution/terminalInteraction" => {
            if cli.verbose_events
                && let Some(summary) = summarize_terminal_interaction(params)
            {
                output.line_stderr(format!("[command-input] {summary}"))?;
            }
        }
        "serverRequest/resolved" => {
            if cli.verbose_events {
                output.line_stderr(format!(
                    "[approval] resolved {}",
                    summarize_server_request_resolved(params)
                ))?;
            }
        }
        "error" => {
            output.line_stderr(format!("[turn-error] {}", summarize_value(params)))?;
        }
        other if other.starts_with("codex/event/") => {
            if other == "codex/event/task_complete" {
                if let Some(message) = get_string(params, &["msg", "last_agent_message"]) {
                    state.last_agent_message = Some(message.to_string());
                }
            } else if cli.verbose_events {
                output.line_stderr(format!(
                    "[event] {other}: {}",
                    if cli.raw_json {
                        serde_json::to_string_pretty(params)?
                    } else {
                        summarize_value(params)
                    }
                ))?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn render_item_started(params: &Value, state: &mut AppState) -> Result<()> {
    let Some(item) = params.get("item") else {
        return Ok(());
    };
    let item_type = get_string(item, &["type"]).unwrap_or("unknown");
    match item_type {
        "commandExecution" => {
            let command = get_string(item, &["command"]).unwrap_or("");
            state.last_status_line = Some(format!("running {}", summarize_text(command)));
        }
        "fileChange" => {
            state.last_status_line = Some(summarize_file_change_paths(item));
        }
        "agentMessage" | "reasoning" => {}
        "mcpToolCall" | "dynamicToolCall" | "collabAgentToolCall" | "webSearch" | "plan" => {
            state.last_status_line = Some(summarize_text(&format!(
                "{} {}",
                humanize_item_type(item_type),
                summarize_tool_item(item_type, item)
            )));
        }
        _ => {}
    }
    Ok(())
}
