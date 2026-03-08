use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;
use crate::state::buffer_item_delta;
use crate::state::buffer_process_delta;
use crate::state::get_string;
use crate::status_views::summarize_value;
use crate::transcript_approval_summary::summarize_terminal_interaction;
use crate::transcript_plan_render::format_plan;

pub(crate) fn handle_buffer_update(
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
