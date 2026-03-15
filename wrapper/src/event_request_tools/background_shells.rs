mod admission;
mod backpressure;

use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;

use crate::output::Output;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AppEvent;
use crate::state::AbandonedAsyncToolRequest;
use crate::state::AppState;

const DEFAULT_BACKGROUND_SHELL_TOOL_TIMEOUT_MS: u64 = 30_000;
const BACKGROUND_SHELL_START_TIMEOUT_MS: u64 = 15_000;
const BACKGROUND_SHELL_REQUEST_TIMEOUT_GRACE_MS: u64 = 5_000;
const MAX_BACKGROUND_SHELL_TOOL_TIMEOUT_MS: u64 = 300_000;

pub(super) fn handle_background_shell_tool_request(
    request: &RpcRequest,
    tool: &str,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<()> {
    admission::handle_background_shell_tool_request(
        request,
        tool,
        resolved_cwd,
        state,
        output,
        writer,
        tx,
    )
}

pub(super) fn background_shell_backpressure_failure(
    tool: &str,
    backlog: usize,
    oldest_summary: &str,
    oldest_context: &str,
    backpressure: serde_json::Value,
) -> serde_json::Value {
    backpressure::background_shell_backpressure_failure(
        tool,
        backlog,
        oldest_summary,
        oldest_context,
        backpressure,
    )
}

pub(super) fn background_shell_backpressure_details(
    resolved_cwd: &str,
    state: &AppState,
    request: Option<(&crate::rpc::RequestId, &AbandonedAsyncToolRequest)>,
) -> serde_json::Value {
    backpressure::background_shell_backpressure_details(resolved_cwd, state, request)
}

pub(super) fn summarize_abandoned_backpressure_context(
    state: &AppState,
    request: &AbandonedAsyncToolRequest,
) -> String {
    backpressure::summarize_abandoned_backpressure_context(state, request)
}
