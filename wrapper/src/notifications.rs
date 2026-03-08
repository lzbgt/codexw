use std::process::ChildStdin;
use std::time::Instant;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::catalog::parse_apps_list;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::prompt::build_continue_prompt;
use crate::prompt::parse_auto_mode_stop;
use crate::requests::send_load_skills;
use crate::requests::send_turn_start;
use crate::rpc::RpcNotification;
use crate::session::render_realtime_item;
use crate::state::AppState;
use crate::state::buffer_item_delta;
use crate::state::buffer_process_delta;
use crate::state::emit_status_line;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::state::thread_id;
use crate::views::format_plan;
use crate::views::humanize_item_type;
use crate::views::render_command_completion;
use crate::views::render_file_change_completion;
use crate::views::render_reasoning_item;
use crate::views::summarize_file_change_paths;
use crate::views::summarize_model_reroute;
use crate::views::summarize_server_request_resolved;
use crate::views::summarize_terminal_interaction;
use crate::views::summarize_thread_status_for_display;
use crate::views::summarize_tool_item;
use crate::views::summarize_value;

pub(crate) fn handle_notification(
    notification: RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    match notification.method.as_str() {
        "thread/started" => {
            if let Some(thread_id) = get_string(&notification.params, &["thread", "id"]) {
                state.thread_id = Some(thread_id.to_string());
            }
        }
        "skills/changed" => {
            send_load_skills(writer, state, resolved_cwd)?;
        }
        "app/list/updated" => {
            state.apps = parse_apps_list(&notification.params);
        }
        "account/updated" => {
            state.account_info = notification.params.get("account").cloned().or_else(|| {
                let auth_mode = notification.params.get("authMode")?.clone();
                let plan_type = notification
                    .params
                    .get("planType")
                    .cloned()
                    .unwrap_or(Value::Null);
                Some(serde_json::json!({
                    "type": auth_mode,
                    "planType": plan_type,
                }))
            });
        }
        "account/rateLimits/updated" => {
            state.rate_limits = notification.params.get("rateLimits").cloned();
        }
        "thread/status/changed" => {
            if let Some(status_line) = summarize_thread_status_for_display(&notification.params) {
                emit_status_line(output, state, status_line)?;
            }
        }
        "thread/tokenUsage/updated" => {
            state.last_token_usage = notification.params.get("tokenUsage").cloned();
        }
        "thread/realtime/started" => {
            state.realtime_active = true;
            state.realtime_started_at = Some(Instant::now());
            state.realtime_session_id =
                get_string(&notification.params, &["sessionId"]).map(ToOwned::to_owned);
            state.realtime_last_error = None;
            let session = state.realtime_session_id.as_deref().unwrap_or("ephemeral");
            output.line_stderr(format!("[realtime] active session={session}"))?;
        }
        "thread/realtime/itemAdded" => {
            if let Some(item) = notification.params.get("item") {
                output.block_stdout("Realtime", &render_realtime_item(item))?;
            }
        }
        "thread/realtime/outputAudio/delta" => {
            if cli.verbose_events {
                output.line_stderr("[realtime] received output audio chunk (not rendered)")?;
            }
        }
        "thread/realtime/error" => {
            let message = get_string(&notification.params, &["message"])
                .unwrap_or("unknown realtime error")
                .to_string();
            state.realtime_last_error = Some(message.clone());
            output.line_stderr(format!("[realtime-error] {message}"))?;
        }
        "thread/realtime/closed" => {
            let reason = get_string(&notification.params, &["reason"]).map(ToOwned::to_owned);
            state.realtime_active = false;
            state.realtime_session_id = None;
            state.realtime_started_at = None;
            if let Some(reason) = reason {
                output.line_stderr(format!("[realtime] closed: {reason}"))?;
            } else {
                output.line_stderr("[realtime] closed")?;
            }
        }
        "turn/started" => {
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.started_turn_count = state.started_turn_count.saturating_add(1);
            if let Some(turn_id) = get_string(&notification.params, &["turn", "id"]) {
                state.active_turn_id = Some(turn_id.to_string());
            }
            state.reset_turn_stream_state();
            state.last_status_line = None;
        }
        "turn/completed" => {
            output.finish_stream()?;
            let status = get_string(&notification.params, &["turn", "status"])
                .unwrap_or("unknown")
                .to_string();
            let turn_id = get_string(&notification.params, &["turn", "id"])
                .unwrap_or("?")
                .to_string();
            state.turn_running = false;
            state.active_turn_id = None;
            state.activity_started_at = None;
            state.last_status_line = None;
            if matches!(
                status.as_str(),
                "completed" | "interrupted" | "failed" | "cancelled"
            ) {
                state.completed_turn_count = state.completed_turn_count.saturating_add(1);
            }
            if status != "completed" {
                output.line_stderr(format!("[turn] completed {turn_id} status={status}"))?;
            }

            if status == "completed" {
                if let Some(message) = state.last_agent_message.clone() {
                    let stop = parse_auto_mode_stop(&message);
                    if state.auto_continue && !stop {
                        let thread_id = thread_id(state)?.to_string();
                        let continue_prompt =
                            build_continue_prompt(state.objective.as_deref(), &message);
                        let submission = build_turn_input(
                            &continue_prompt,
                            resolved_cwd,
                            &[],
                            &[],
                            &state.apps,
                            &state.plugins,
                            &state.skills,
                        );
                        output.line_stderr("[auto] continuing remaining work")?;
                        send_turn_start(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            thread_id,
                            submission,
                            true,
                        )?;
                    } else if stop {
                        output.line_stderr("[ready] stop marker observed")?;
                    } else {
                        output.line_stderr("[ready]")?;
                    }
                } else {
                    output.line_stderr("[ready]")?;
                }
            } else {
                state.last_agent_message = None;
                output.line_stderr("[ready]")?;
            }
        }
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
    Ok(())
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
