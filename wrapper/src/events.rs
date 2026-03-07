use std::process::ChildStdin;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::catalog::parse_apps_list;
use crate::catalog::parse_skills_list;
use crate::choose_command_approval_decision;
use crate::choose_first_allowed_decision;
use crate::history::render_resumed_history;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::prompt::build_continue_prompt;
use crate::prompt::parse_auto_mode_stop;
use crate::requests::*;
use crate::rpc;
use crate::rpc::IncomingMessage;
use crate::rpc::OutgoingErrorObject;
use crate::rpc::OutgoingErrorResponse;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcNotification;
use crate::rpc::RpcRequest;
use crate::rpc::RpcResponse;
use crate::runtime::StartMode;
use crate::session::CollaborationModeAction;
use crate::session::ModelsAction;
use crate::session::apply_collaboration_mode_action;
use crate::session::apply_models_action;
use crate::session::extract_collaboration_mode_presets;
use crate::session::render_realtime_item;
use crate::state::AppState;
use crate::state::buffer_item_delta;
use crate::state::buffer_process_delta;
use crate::state::emit_status_line;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::state::thread_id;
use crate::views::build_tool_user_input_response;
use crate::views::extract_file_search_paths;
use crate::views::extract_thread_ids;
use crate::views::format_plan;
use crate::views::humanize_item_type;
use crate::views::render_command_completion;
use crate::views::render_config_snapshot;
use crate::views::render_experimental_features_list;
use crate::views::render_file_change_completion;
use crate::views::render_fuzzy_file_search_results;
use crate::views::render_local_command_completion;
use crate::views::render_mcp_server_list;
use crate::views::render_reasoning_item;
use crate::views::render_thread_list;
use crate::views::summarize_command_approval_request;
use crate::views::summarize_file_change_paths;
use crate::views::summarize_generic_approval_request;
use crate::views::summarize_model_reroute;
use crate::views::summarize_server_request_resolved;
use crate::views::summarize_terminal_interaction;
use crate::views::summarize_thread_status_for_display;
use crate::views::summarize_tool_item;
use crate::views::summarize_tool_request;
use crate::views::summarize_value;

pub(crate) fn process_server_line(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if state.raw_json {
        output.line_stderr(format!("[json] {line}"))?;
    }
    match rpc::parse_line(&line) {
        Ok(IncomingMessage::Response(response)) => {
            handle_response(
                response,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
                start_after_initialize,
            )?;
        }
        Ok(IncomingMessage::Request(request)) => {
            handle_server_request(request, cli, output, writer)?;
        }
        Ok(IncomingMessage::Notification(notification)) => {
            handle_notification(notification, cli, resolved_cwd, state, output, writer)?;
        }
        Err(err) => {
            output.line_stderr(format!("[session] ignored malformed server line: {err}"))?;
        }
    }
    Ok(())
}

fn handle_response(
    response: RpcResponse,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    let pending = state.pending.remove(&response.id);
    if let Some(error) = response.error {
        return handle_response_error(error, pending, state, output);
    }

    let Some(pending) = pending else {
        return Ok(());
    };

    match pending {
        PendingRequest::Initialize => {
            send_initialized(writer)?;
            output.line_stderr("[session] connected")?;
            if let Some(start_mode) = start_after_initialize.take() {
                match start_mode.resume_thread_id {
                    Some(thread_id) => {
                        output.line_stderr(format!("[thread] resume {thread_id}"))?;
                        send_thread_resume(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            thread_id,
                            start_mode.initial_prompt,
                        )?
                    }
                    None => {
                        output.line_stderr("[thread] create")?;
                        send_thread_start(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            start_mode.initial_prompt,
                        )?
                    }
                }
            }
            send_load_apps(writer, state)?;
            send_load_skills(writer, state, resolved_cwd)?;
            send_load_models(writer, state, ModelsAction::CacheOnly)?;
            send_load_collaboration_modes(writer, state, CollaborationModeAction::CacheOnly)?;
            send_load_account(writer, state)?;
            send_load_rate_limits(writer, state)?;
        }
        PendingRequest::StartThread { initial_prompt } => {
            state.pending_thread_switch = false;
            state.reset_thread_context();
            let thread_id = get_string(&response.result, &["thread", "id"])
                .context("thread/start missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] started {thread_id}"))?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    &text,
                    resolved_cwd,
                    &[],
                    &[],
                    &state.apps,
                    &state.plugins,
                    &state.skills,
                );
                send_turn_start(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id,
                    submission,
                    false,
                )?;
            }
        }
        PendingRequest::ResumeThread { initial_prompt } => {
            state.pending_thread_switch = false;
            state.reset_thread_context();
            let thread_id = get_string(&response.result, &["thread", "id"])
                .context("thread/resume missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] resumed {thread_id}"))?;
            render_resumed_history(&response.result, state, output)?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    &text,
                    resolved_cwd,
                    &[],
                    &[],
                    &state.apps,
                    &state.plugins,
                    &state.skills,
                );
                send_turn_start(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id,
                    submission,
                    false,
                )?;
            }
        }
        PendingRequest::ForkThread { initial_prompt } => {
            state.pending_thread_switch = false;
            state.reset_thread_context();
            let thread_id = get_string(&response.result, &["thread", "id"])
                .context("thread/fork missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] forked to {thread_id}"))?;
            render_resumed_history(&response.result, state, output)?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    &text,
                    resolved_cwd,
                    &[],
                    &[],
                    &state.apps,
                    &state.plugins,
                    &state.skills,
                );
                send_turn_start(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id,
                    submission,
                    false,
                )?;
            }
        }
        PendingRequest::CompactThread => {
            output.line_stderr("[thread] compaction requested")?;
        }
        PendingRequest::RenameThread { name } => {
            output.line_stderr(format!("[thread] renamed to {}", summarize_text(&name)))?;
        }
        PendingRequest::CleanBackgroundTerminals => {
            output.line_stderr("[thread] background terminal cleanup requested")?;
        }
        PendingRequest::StartRealtime { prompt } => {
            state.realtime_prompt = Some(prompt);
            output.line_stderr("[realtime] start requested")?;
        }
        PendingRequest::AppendRealtimeText { text } => {
            output.line_stderr(format!("[realtime] sent {}", summarize_text(&text)))?;
        }
        PendingRequest::StopRealtime => {
            output.line_stderr("[realtime] stop requested")?;
        }
        PendingRequest::StartReview { target_description } => {
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.reset_turn_stream_state();
            output.line_stderr(format!(
                "[review] started {}",
                summarize_text(&target_description)
            ))?;
        }
        PendingRequest::StartTurn { auto_generated } => {
            let turn_id = get_string(&response.result, &["turn", "id"])
                .context("turn/start missing turn.id")?
                .to_string();
            state.active_turn_id = Some(turn_id.clone());
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.reset_turn_stream_state();
            if auto_generated {
                output.line_stderr("[auto] starting follow-up turn")?;
            }
        }
        PendingRequest::SteerTurn { display_text } => {
            let turn_id = get_string(&response.result, &["turnId"])
                .context("turn/steer missing turnId")?
                .to_string();
            state.active_turn_id = Some(turn_id);
            output.line_stderr(format!("[steer] {}", summarize_text(&display_text)))?;
        }
        PendingRequest::InterruptTurn => {
            output.line_stderr("[interrupt] requested")?;
        }
        PendingRequest::LoadApps => {
            state.apps = parse_apps_list(&response.result);
        }
        PendingRequest::LoadSkills => {
            state.skills = parse_skills_list(&response.result, resolved_cwd);
        }
        PendingRequest::LoadAccount => {
            state.account_info = response.result.get("account").cloned();
        }
        PendingRequest::LogoutAccount => {
            state.account_info = None;
            state.rate_limits = None;
            output.line_stderr("[session] logged out")?;
            send_load_account(writer, state)?;
            send_load_rate_limits(writer, state)?;
        }
        PendingRequest::UploadFeedback { classification } => {
            let tracking_thread = get_string(&response.result, &["threadId"]).unwrap_or("-");
            output.line_stderr(format!(
                "[feedback] submitted {} feedback; tracking thread {}",
                summarize_text(&classification),
                tracking_thread
            ))?;
        }
        PendingRequest::LoadRateLimits => {
            state.rate_limits = response.result.get("rateLimits").cloned();
        }
        PendingRequest::LoadModels { action } => {
            apply_models_action(cli, state, action, &response.result, output)?;
        }
        PendingRequest::LoadExperimentalFeatures => {
            output.block_stdout(
                "Experimental features",
                &render_experimental_features_list(&response.result),
            )?;
        }
        PendingRequest::LoadCollaborationModes { action } => {
            state.collaboration_modes = extract_collaboration_mode_presets(&response.result);
            apply_collaboration_mode_action(state, action, output)?;
        }
        PendingRequest::LoadConfig => {
            output.block_stdout("Config", &render_config_snapshot(&response.result))?;
        }
        PendingRequest::LoadMcpServers => {
            output.block_stdout("MCP servers", &render_mcp_server_list(&response.result))?;
        }
        PendingRequest::ListThreads { search_term } => {
            state.last_listed_thread_ids = extract_thread_ids(&response.result);
            output.block_stdout(
                "Threads",
                &render_thread_list(&response.result, search_term.as_deref()),
            )?;
        }
        PendingRequest::ExecCommand {
            process_id,
            command,
        } => {
            let exit_code = response
                .result
                .get("exitCode")
                .and_then(Value::as_i64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let buffer = state
                .process_output_buffers
                .remove(&process_id)
                .unwrap_or_default();
            let stdout = if buffer.stdout.trim().is_empty() {
                get_string(&response.result, &["stdout"])
                    .unwrap_or("")
                    .to_string()
            } else {
                buffer.stdout
            };
            let stderr = if buffer.stderr.trim().is_empty() {
                get_string(&response.result, &["stderr"])
                    .unwrap_or("")
                    .to_string()
            } else {
                buffer.stderr
            };
            state.active_exec_process_id = None;
            state.activity_started_at = None;
            state.last_status_line = None;
            output.block_stdout(
                "Local command",
                &render_local_command_completion(&command, &exit_code, &stdout, &stderr),
            )?;
        }
        PendingRequest::TerminateExecCommand { process_id } => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.activity_started_at = None;
                output.line_stderr("[interrupt] local command termination requested")?;
            }
        }
        PendingRequest::FuzzyFileSearch { query } => {
            let files = response
                .result
                .get("files")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            state.last_file_search_paths = extract_file_search_paths(&files);
            let rendered = render_fuzzy_file_search_results(&query, files.as_slice());
            output.block_stdout("File mentions", &rendered)?;
        }
    }

    Ok(())
}

fn handle_response_error(
    error: Value,
    pending: Option<PendingRequest>,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    match pending {
        Some(PendingRequest::LoadRateLimits) => {
            output.line_stderr("[session] rate limits unavailable for the current account")?;
        }
        Some(PendingRequest::LoadAccount) => {
            output.line_stderr("[session] account details unavailable from app-server")?;
        }
        Some(PendingRequest::LoadModels { .. }) => {
            output.line_stderr("[session] model metadata unavailable from app-server")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::LoadCollaborationModes { action }) => {
            if !matches!(action, CollaborationModeAction::CacheOnly) {
                output.line_stderr(
                    "[session] collaboration modes are unavailable from this app-server build",
                )?;
                output.line_stderr(format!(
                    "[server-error] {}",
                    serde_json::to_string_pretty(&error)?
                ))?;
            }
        }
        Some(PendingRequest::LogoutAccount) => {
            output.line_stderr("[session] logout failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::StartRealtime { .. })
        | Some(PendingRequest::AppendRealtimeText { .. })
        | Some(PendingRequest::StopRealtime) => {
            output.line_stderr("[realtime] request failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::UploadFeedback { classification }) => {
            output.line_stderr(format!(
                "[feedback] failed to submit {} feedback",
                summarize_text(&classification)
            ))?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::StartThread { .. })
        | Some(PendingRequest::ResumeThread { .. })
        | Some(PendingRequest::ForkThread { .. }) => {
            state.pending_thread_switch = false;
            output.line_stderr("[thread] failed to switch threads")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::ExecCommand { process_id, .. }) => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            state.process_output_buffers.remove(&process_id);
            state.last_status_line = None;
            output.line_stderr("[command] failed to start local command")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::TerminateExecCommand { process_id }) => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            output.line_stderr("[command] failed to terminate local command cleanly")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        _ => {
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
    }
    Ok(())
}

fn handle_server_request(
    request: RpcRequest,
    cli: &Cli,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    match request.method.as_str() {
        "item/commandExecution/requestApproval" => {
            let decision_value = choose_command_approval_decision(&request.params, cli.yolo);
            output.line_stderr(format!(
                "[approval] {}",
                summarize_command_approval_request(&request.params, &decision_value)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({"decision": decision_value}),
                },
            )?;
        }
        "item/fileChange/requestApproval" | "execCommandApproval" | "applyPatchApproval" => {
            let decision = params_auto_approval_result(&request.params);
            output.line_stderr(format!(
                "[approval] {}",
                summarize_generic_approval_request(&request.params, &request.method)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: decision,
                },
            )?;
        }
        "tool/requestUserInput" | "item/tool/requestUserInput" => {
            let result = build_tool_user_input_response(&request.params);
            output.line_stderr(format!(
                "[input-request] auto-answered: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result,
                },
            )?;
        }
        "mcpServer/elicitation/request" => {
            output.line_stderr(format!(
                "[input-request] auto-cancelled MCP elicitation: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({"action": "cancel", "content": Value::Null}),
                },
            )?;
        }
        "item/tool/call" => {
            output.line_stderr(format!(
                "[tool] unsupported dynamic tool call: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({
                        "contentItems": [
                            {
                                "type": "inputText",
                                "text": "codexw does not implement dynamic tool calls"
                            }
                        ],
                        "success": false
                    }),
                },
            )?;
        }
        _ => {
            if cli.verbose_events || cli.raw_json {
                output.line_stderr(format!(
                    "[server-request] {}: {}",
                    request.method,
                    if cli.raw_json {
                        serde_json::to_string_pretty(&request.params)?
                    } else {
                        summarize_value(&request.params)
                    }
                ))?;
            }
            send_json(
                writer,
                &OutgoingErrorResponse {
                    id: request.id,
                    error: OutgoingErrorObject {
                        code: -32601,
                        message: format!("codexw does not implement {}", request.method),
                        data: None,
                    },
                },
            )?;
        }
    }
    Ok(())
}

pub(crate) fn params_auto_approval_result(params: &Value) -> Value {
    if let Some(decisions) = params.get("availableDecisions").and_then(Value::as_array)
        && let Some(decision) = choose_first_allowed_decision(decisions)
    {
        return json!({"decision": decision});
    }
    json!({"decision": "accept"})
}

fn handle_notification(
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
                Some(json!({
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
