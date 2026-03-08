use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::response_thread_switch::handle_forked_thread;
use crate::response_thread_switch::handle_resumed_thread;
use crate::response_thread_switch::handle_started_thread;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) fn handle_thread_session_response(
    pending: &crate::requests::PendingRequest,
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match pending {
        crate::requests::PendingRequest::StartThread { initial_prompt } => {
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
        crate::requests::PendingRequest::ResumeThread { initial_prompt } => {
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
        crate::requests::PendingRequest::ForkThread { initial_prompt } => {
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
        crate::requests::PendingRequest::CompactThread => {
            output.line_stderr("[thread] compaction requested")?;
        }
        crate::requests::PendingRequest::RenameThread { name } => {
            output.line_stderr(format!("[thread] renamed to {}", summarize_text(name)))?;
        }
        crate::requests::PendingRequest::CleanBackgroundTerminals => {
            output.line_stderr("[thread] background terminal cleanup requested")?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
