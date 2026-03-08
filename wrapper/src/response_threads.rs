use std::process::ChildStdin;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::history::render_resumed_history;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::requests::send_turn_start;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::transcript_render::render_local_command_completion;

pub(crate) fn handle_thread_response_success(
    result: &Value,
    pending: &PendingRequest,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match pending {
        PendingRequest::StartThread { initial_prompt } => {
            state.pending_thread_switch = false;
            state.reset_thread_context();
            let thread_id = get_string(result, &["thread", "id"])
                .context("thread/start missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] started {thread_id}"))?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    text,
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
            let thread_id = get_string(result, &["thread", "id"])
                .context("thread/resume missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] resumed {thread_id}"))?;
            render_resumed_history(result, state, output)?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    text,
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
            let thread_id = get_string(result, &["thread", "id"])
                .context("thread/fork missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] forked to {thread_id}"))?;
            render_resumed_history(result, state, output)?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    text,
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
            output.line_stderr(format!("[thread] renamed to {}", summarize_text(name)))?;
        }
        PendingRequest::CleanBackgroundTerminals => {
            output.line_stderr("[thread] background terminal cleanup requested")?;
        }
        PendingRequest::StartRealtime { prompt } => {
            state.realtime_prompt = Some(prompt.clone());
            output.line_stderr("[realtime] start requested")?;
        }
        PendingRequest::AppendRealtimeText { text } => {
            output.line_stderr(format!("[realtime] sent {}", summarize_text(text)))?;
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
                summarize_text(target_description)
            ))?;
        }
        PendingRequest::StartTurn { auto_generated } => {
            let turn_id = get_string(result, &["turn", "id"])
                .context("turn/start missing turn.id")?
                .to_string();
            state.active_turn_id = Some(turn_id.clone());
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.reset_turn_stream_state();
            if *auto_generated {
                output.line_stderr("[auto] starting follow-up turn")?;
            }
        }
        PendingRequest::SteerTurn { display_text } => {
            let turn_id = get_string(result, &["turnId"])
                .context("turn/steer missing turnId")?
                .to_string();
            state.active_turn_id = Some(turn_id);
            output.line_stderr(format!("[steer] {}", summarize_text(display_text)))?;
        }
        PendingRequest::InterruptTurn => {
            output.line_stderr("[interrupt] requested")?;
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
                .remove(process_id)
                .unwrap_or_default();
            let stdout = if buffer.stdout.trim().is_empty() {
                get_string(result, &["stdout"]).unwrap_or("").to_string()
            } else {
                buffer.stdout
            };
            let stderr = if buffer.stderr.trim().is_empty() {
                get_string(result, &["stderr"]).unwrap_or("").to_string()
            } else {
                buffer.stderr
            };
            state.active_exec_process_id = None;
            state.activity_started_at = None;
            state.last_status_line = None;
            output.block_stdout(
                "Local command",
                &render_local_command_completion(command, &exit_code, &stdout, &stderr),
            )?;
        }
        PendingRequest::TerminateExecCommand { process_id } => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.activity_started_at = None;
                output.line_stderr("[interrupt] local command termination requested")?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
