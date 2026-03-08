use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::rpc::RpcResponse;
use crate::runtime_process::StartMode;
use crate::state::AppState;

#[path = "response_error_runtime.rs"]
mod response_error_runtime;
#[path = "response_error_session.rs"]
mod response_error_session;
#[path = "response_success.rs"]
mod response_success;

use response_success::handle_response_success;

pub(crate) fn handle_response(
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

    handle_response_success(
        response.result,
        pending,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        start_after_initialize,
    )
}

pub(crate) fn handle_response_error(
    error: Value,
    pending: Option<PendingRequest>,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    if let Some(pending) = pending.as_ref()
        && (response_error_session::handle_session_error(&error, pending, state, output)?
            || response_error_runtime::handle_runtime_error(&error, pending, state, output)?)
    {
        return Ok(());
    }
    output.line_stderr(format!(
        "[server-error] {}",
        serde_json::to_string_pretty(&error)?
    ))?;
    Ok(())
}
