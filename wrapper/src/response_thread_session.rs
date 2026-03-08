use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

#[path = "response_thread_maintenance.rs"]
mod response_thread_maintenance;

use self::response_thread_maintenance::handle_thread_maintenance_response;
use crate::Cli;
use crate::output::Output;
use crate::response_thread_loaded::handle_forked_thread;
use crate::response_thread_loaded::handle_resumed_thread;
use crate::response_thread_loaded::handle_started_thread;
use crate::state::AppState;

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
        _ => return handle_thread_maintenance_response(pending, output),
    }
    Ok(true)
}
