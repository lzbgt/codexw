use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::rpc::RpcNotification;
use crate::state::AppState;
use crate::state::buffer_item_delta;
use crate::state::buffer_process_delta;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::status_views::summarize_value;
use crate::transcript_render::format_plan;
use crate::transcript_render::render_command_completion;
use crate::transcript_render::render_file_change_completion;
use crate::transcript_render::render_reasoning_item;
use crate::transcript_summary::humanize_item_type;
use crate::transcript_summary::summarize_file_change_paths;
use crate::transcript_summary::summarize_model_reroute;
use crate::transcript_summary::summarize_server_request_resolved;
use crate::transcript_summary::summarize_terminal_interaction;
use crate::transcript_summary::summarize_tool_item;

pub(crate) fn handle_turn_item_notification(
    notification: RpcNotification,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match notification.method.as_str() {
        "command/exec/outputDelta" => {
            buffer_process_delta(&mut state.process_output_buffers, &notification.params);
        }
        "turn/diff/updated" => {
            state.last_turn_diff =
                get_string(&notification.params, &["diff"]).map(ToOwned::to_owned);
            if cli.verbose_events
                && let Some(diff) = get_string(&notification.params, &["diff"])
            {
                output.line_stdout("[diff]")?;
                output.line_stdout(diff)?;
            }
        }
        "turn/plan/updated" => {
            let plan_text = format_plan(&notification.params);
            if !plan_text.is_empty() {
                output.line_stdout("[plan]")?;
                output.line_stdout(plan_text)?;
            }
        }
        "model/rerouted" => {
            output.line_stderr(format!(
                "[model] {}",
                summarize_model_reroute(&notification.params)
            ))?;
        }
        "item/started" => render_item_started(&notification.params, state)?,
        "item/completed" => render_item_completed(&notification.params, state, output)?,
        "item/agentMessage/delta"
        | "item/reasoning/summaryTextDelta"
        | "item/reasoning/textDelta"
        | "item/reasoning/summaryPartAdded" => {}
        "item/commandExecution/outputDelta" => {
            buffer_item_delta(&mut state.command_output_buffers, &notification.params)
        }
        "item/fileChange/outputDelta" => {
            buffer_item_delta(&mut state.file_output_buffers, &notification.params)
        }
        "item/commandExecution/terminalInteraction" => {
            if cli.verbose_events
                && let Some(summary) = summarize_terminal_interaction(&notification.params)
            {
                output.line_stderr(format!("[command-input] {summary}"))?;
            }
        }
        "serverRequest/resolved" => {
            if cli.verbose_events {
                output.line_stderr(format!(
                    "[approval] resolved {}",
                    summarize_server_request_resolved(&notification.params)
                ))?;
            }
        }
        "error" => {
            output.line_stderr(format!(
                "[turn-error] {}",
                summarize_value(&notification.params)
            ))?;
        }
        other if other.starts_with("codex/event/") => {
            if other == "codex/event/task_complete" {
                if let Some(message) =
                    get_string(&notification.params, &["msg", "last_agent_message"])
                {
                    state.last_agent_message = Some(message.to_string());
                }
            } else if cli.verbose_events {
                output.line_stderr(format!(
                    "[event] {other}: {}",
                    if cli.raw_json {
                        serde_json::to_string_pretty(&notification.params)?
                    } else {
                        summarize_value(&notification.params)
                    }
                ))?;
            }
        }
        other => {
            if cli.verbose_events {
                output.line_stderr(format!(
                    "[event] {other}: {}",
                    if cli.raw_json {
                        serde_json::to_string_pretty(&notification.params)?
                    } else {
                        summarize_value(&notification.params)
                    }
                ))?;
            }
        }
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
            state.last_status_line = Some(format!(
                "{}",
                summarize_text(&format!(
                    "{} {}",
                    humanize_item_type(item_type),
                    summarize_tool_item(item_type, item)
                ))
            ));
        }
        _ => {}
    }
    Ok(())
}

fn render_item_completed(params: &Value, state: &mut AppState, output: &mut Output) -> Result<()> {
    let Some(item) = params.get("item") else {
        return Ok(());
    };
    let item_type = get_string(item, &["type"]).unwrap_or("unknown");
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
            let rendered =
                render_command_completion(command, status, &exit_code, full_output.as_deref());
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
            let rendered = render_file_change_completion(item, status, delta_output.as_deref());
            output.block_stdout("File changes complete", &rendered)?;
        }
        "reasoning" => {
            output.finish_stream()?;
            let rendered = render_reasoning_item(item);
            if !rendered.is_empty() {
                output.block_stdout("Thinking", &rendered)?;
            }
        }
        "mcpToolCall" | "dynamicToolCall" | "collabAgentToolCall" | "webSearch" | "plan" => {
            output.finish_stream()?;
            output.block_stdout(
                &format!("{} complete", humanize_item_type(item_type)),
                &summarize_tool_item(item_type, item),
            )?;
        }
        _ => {}
    }
    Ok(())
}
