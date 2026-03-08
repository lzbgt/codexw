use anyhow::Result;

use crate::Cli;
use crate::model_session::ModelsAction;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::runtime::StartMode;
use crate::state::AppState;
use serde_json::Value;
use std::process::ChildStdin;

#[path = "response_bootstrap.rs"]
mod response_bootstrap;
#[path = "response_threads.rs"]
mod response_threads;

use response_bootstrap::handle_bootstrap_response_success;
use response_threads::handle_thread_response_success;

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

    if handle_thread_response_success(&result, &pending, cli, resolved_cwd, state, output, writer)?
    {
        return Ok(());
    }

    Ok(())
}
