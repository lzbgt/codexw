use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;

use crate::output::Output;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AppEvent;
use crate::state::AppState;

mod background_shells;
mod dynamic;
mod input;

#[cfg(test)]
mod tests;

pub(crate) fn handle_tool_request(
    request: &RpcRequest,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<bool> {
    if input::try_handle_tool_request(request, output, writer)? {
        return Ok(true);
    }
    if request.method != "item/tool/call" {
        return Ok(false);
    }

    let tool = request
        .params
        .get("tool")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("dynamic tool");
    if crate::client_dynamic_tools::is_background_shell_tool(tool) {
        background_shells::handle_background_shell_tool_request(
            request,
            tool,
            resolved_cwd,
            state,
            output,
            writer,
            tx,
        )?;
    } else {
        dynamic::handle_dynamic_tool_request(request, tool, resolved_cwd, state, output, writer)?;
    }
    Ok(true)
}
