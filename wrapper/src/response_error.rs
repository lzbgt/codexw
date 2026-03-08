use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::requests::PendingRequest;
use crate::state::AppState;

#[path = "response_error_runtime.rs"]
mod response_error_runtime;
#[path = "response_error_session.rs"]
mod response_error_session;

pub(crate) fn handle_response_error_impl(
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
