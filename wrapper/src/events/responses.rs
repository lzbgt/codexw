use anyhow::Result;
use serde_json::Value;
use std::process::ChildStdin;

#[path = "responses/bootstrap.rs"]
mod bootstrap;
#[path = "responses/thread.rs"]
mod thread;

use crate::Cli;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::response_error_runtime::handle_runtime_error;
use crate::response_error_session::handle_session_error;
use crate::response_thread_runtime::handle_thread_runtime_response;
use crate::rpc::RpcResponse;
use crate::runtime_process::StartMode;
use crate::state::AppState;
use bootstrap::handle_bootstrap_response_success;
use thread::handle_thread_session_response;

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

fn handle_response_error(
    error: Value,
    pending: Option<PendingRequest>,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    if let Some(pending) = pending.as_ref()
        && (handle_session_error(&error, pending, state, output)?
            || handle_runtime_error(&error, pending, state, output)?)
    {
        return Ok(());
    }
    output.line_stderr(format!(
        "[server-error] {}",
        serde_json::to_string_pretty(&error)?
    ))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_response_success(
    result: Value,
    pending: PendingRequest,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if handle_bootstrap_response_success(
        &result,
        &pending,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        start_after_initialize,
    )? {
        return Ok(());
    }

    if handle_thread_session_response(&pending, &result, cli, resolved_cwd, state, output, writer)?
    {
        return Ok(());
    }

    if handle_thread_runtime_response(&pending, &result, cli, state, output)? {
        return Ok(());
    }

    Ok(())
}
