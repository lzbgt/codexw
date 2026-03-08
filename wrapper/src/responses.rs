use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::rpc::RpcResponse;
use crate::runtime::StartMode;
use crate::state::AppState;

#[path = "response_error.rs"]
mod response_error;
#[path = "response_success.rs"]
mod response_success;

use response_error::handle_response_error_impl;
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
    handle_response_error_impl(error, pending, state, output)
}
