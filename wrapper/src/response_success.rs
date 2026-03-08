use std::process::ChildStdin;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::catalog::parse_apps_list;
use crate::catalog::parse_skills_list;
use crate::catalog_views::extract_file_search_paths;
use crate::catalog_views::extract_thread_ids;
use crate::catalog_views::render_experimental_features_list;
use crate::catalog_views::render_fuzzy_file_search_results;
use crate::catalog_views::render_mcp_server_list;
use crate::catalog_views::render_thread_list;
use crate::collaboration::CollaborationModeAction;
use crate::collaboration::apply_collaboration_mode_action;
use crate::collaboration::extract_collaboration_mode_presets;
use crate::history::render_resumed_history;
use crate::input::build_turn_input;
use crate::model_session::ModelsAction;
use crate::model_session::apply_models_action;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::requests::send_initialized;
use crate::requests::send_load_account;
use crate::requests::send_load_apps;
use crate::requests::send_load_collaboration_modes;
use crate::requests::send_load_models;
use crate::requests::send_load_rate_limits;
use crate::requests::send_load_skills;
use crate::requests::send_thread_resume;
use crate::requests::send_thread_start;
use crate::requests::send_turn_start;
use crate::runtime::StartMode;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::status_views::render_config_snapshot;
use crate::transcript_render::render_local_command_completion;

pub(crate) fn handle_response_success(
    result: Value,
    pending: PendingRequest,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
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
            let thread_id = get_string(&result, &["thread", "id"])
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
            let thread_id = get_string(&result, &["thread", "id"])
                .context("thread/resume missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] resumed {thread_id}"))?;
            render_resumed_history(&result, state, output)?;
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
            let thread_id = get_string(&result, &["thread", "id"])
                .context("thread/fork missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] forked to {thread_id}"))?;
            render_resumed_history(&result, state, output)?;
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
            let turn_id = get_string(&result, &["turn", "id"])
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
            let turn_id = get_string(&result, &["turnId"])
                .context("turn/steer missing turnId")?
                .to_string();
            state.active_turn_id = Some(turn_id);
            output.line_stderr(format!("[steer] {}", summarize_text(&display_text)))?;
        }
        PendingRequest::InterruptTurn => {
            output.line_stderr("[interrupt] requested")?;
        }
        PendingRequest::LoadApps => {
            state.apps = parse_apps_list(&result);
        }
        PendingRequest::LoadSkills => {
            state.skills = parse_skills_list(&result, resolved_cwd);
        }
        PendingRequest::LoadAccount => {
            state.account_info = result.get("account").cloned();
        }
        PendingRequest::LogoutAccount => {
            state.account_info = None;
            state.rate_limits = None;
            output.line_stderr("[session] logged out")?;
            send_load_account(writer, state)?;
            send_load_rate_limits(writer, state)?;
        }
        PendingRequest::UploadFeedback { classification } => {
            let tracking_thread = get_string(&result, &["threadId"]).unwrap_or("-");
            output.line_stderr(format!(
                "[feedback] submitted {} feedback; tracking thread {}",
                summarize_text(&classification),
                tracking_thread
            ))?;
        }
        PendingRequest::LoadRateLimits => {
            state.rate_limits = result.get("rateLimits").cloned();
        }
        PendingRequest::LoadModels { action } => {
            apply_models_action(cli, state, action, &result, output)?;
        }
        PendingRequest::LoadExperimentalFeatures => {
            output.block_stdout(
                "Experimental features",
                &render_experimental_features_list(&result),
            )?;
        }
        PendingRequest::LoadCollaborationModes { action } => {
            state.collaboration_modes = extract_collaboration_mode_presets(&result);
            apply_collaboration_mode_action(state, action, output)?;
        }
        PendingRequest::LoadConfig => {
            output.block_stdout("Config", &render_config_snapshot(&result))?;
        }
        PendingRequest::LoadMcpServers => {
            output.block_stdout("MCP servers", &render_mcp_server_list(&result))?;
        }
        PendingRequest::ListThreads { search_term } => {
            state.last_listed_thread_ids = extract_thread_ids(&result);
            output.block_stdout(
                "Threads",
                &render_thread_list(&result, search_term.as_deref()),
            )?;
        }
        PendingRequest::ExecCommand {
            process_id,
            command,
        } => {
            let exit_code = result
                .get("exitCode")
                .and_then(Value::as_i64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let buffer = state
                .process_output_buffers
                .remove(&process_id)
                .unwrap_or_default();
            let stdout = if buffer.stdout.trim().is_empty() {
                get_string(&result, &["stdout"]).unwrap_or("").to_string()
            } else {
                buffer.stdout
            };
            let stderr = if buffer.stderr.trim().is_empty() {
                get_string(&result, &["stderr"]).unwrap_or("").to_string()
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
            let files = result
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
