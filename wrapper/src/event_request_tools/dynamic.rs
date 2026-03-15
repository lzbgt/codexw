use anyhow::Result;
use std::process::ChildStdin;

use crate::client_dynamic_tools::execute_dynamic_tool_call_with_state;
use crate::client_dynamic_tools::legacy_workspace_tool_failure_notice;
use crate::client_dynamic_tools::legacy_workspace_tool_notice;
use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::state::AppState;
use crate::transcript_approval_summary::summarize_tool_request;

pub(super) fn handle_dynamic_tool_request(
    request: &RpcRequest,
    tool: &str,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    let result = execute_dynamic_tool_call_with_state(&request.params, resolved_cwd, state);
    let success = result
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if let Some(notice) = legacy_workspace_tool_notice(tool) {
        output.line_stderr(notice)?;
    }
    if let Some(notice) = legacy_workspace_tool_failure_notice(tool, &result) {
        output.line_stderr(notice)?;
    }
    output.line_stderr(format!(
        "[tool] dynamic tool {}: {}",
        if success { "completed" } else { "failed" },
        summarize_tool_request(&request.params)
    ))?;
    send_json(
        writer,
        &OutgoingResponse {
            id: request.id.clone(),
            result,
        },
    )?;
    Ok(())
}
