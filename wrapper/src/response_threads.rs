use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::state::AppState;

#[path = "response_thread_runtime.rs"]
mod response_thread_runtime;
#[path = "response_thread_session.rs"]
mod response_thread_session;

pub(crate) fn handle_thread_response_success(
    result: &Value,
    pending: &PendingRequest,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    if response_thread_session::handle_thread_session_response(
        pending,
        result,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
    )? {
        return Ok(true);
    }
    response_thread_runtime::handle_thread_runtime_response(pending, result, state, output)
}
