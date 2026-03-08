use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::response_thread_activity::handle_exec_command;
use crate::response_thread_activity::handle_realtime_append;
use crate::response_thread_activity::handle_realtime_start;
use crate::response_thread_activity::handle_realtime_stop;
use crate::response_thread_activity::handle_review_start;
use crate::response_thread_activity::handle_terminate_exec_command;
use crate::response_thread_activity::handle_turn_interrupt;
use crate::response_thread_activity::handle_turn_start;
use crate::response_thread_activity::handle_turn_steer;
use crate::response_thread_switch::handle_forked_thread;
use crate::response_thread_switch::handle_resumed_thread;
use crate::response_thread_switch::handle_started_thread;
use crate::state::AppState;
use crate::state::summarize_text;

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
            handle_started_thread(
                result,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
                initial_prompt.as_deref(),
            )?;
        }
        PendingRequest::ResumeThread { initial_prompt } => {
            handle_resumed_thread(
                result,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
                initial_prompt.as_deref(),
            )?;
        }
        PendingRequest::ForkThread { initial_prompt } => {
            handle_forked_thread(
                result,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
                initial_prompt.as_deref(),
            )?;
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
            handle_realtime_start(state, output, prompt)?;
        }
        PendingRequest::AppendRealtimeText { text } => {
            handle_realtime_append(output, text)?;
        }
        PendingRequest::StopRealtime => {
            handle_realtime_stop(output)?;
        }
        PendingRequest::StartReview { target_description } => {
            handle_review_start(state, output, target_description)?;
        }
        PendingRequest::StartTurn { auto_generated } => {
            handle_turn_start(result, state, output, *auto_generated)?;
        }
        PendingRequest::SteerTurn { display_text } => {
            handle_turn_steer(result, state, output, display_text)?;
        }
        PendingRequest::InterruptTurn => {
            handle_turn_interrupt(output)?;
        }
        PendingRequest::ExecCommand {
            process_id,
            command,
        } => {
            handle_exec_command(result, state, output, process_id, command)?;
        }
        PendingRequest::TerminateExecCommand { process_id } => {
            handle_terminate_exec_command(state, output, process_id)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
