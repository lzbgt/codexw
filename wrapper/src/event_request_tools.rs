use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;

use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AppEvent;
use crate::state::AppState;

mod input;

#[cfg(test)]
mod tests;

pub(crate) fn handle_tool_request(
    request: &RpcRequest,
    _resolved_cwd: &str,
    _state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    _tx: &mpsc::Sender<AppEvent>,
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
        .unwrap_or("tool");
    output.line_stderr(format!("[tool] rejected tool request: {tool}"))?;
    send_json(
        writer,
        &OutgoingResponse {
            id: request.id.clone(),
            result: serde_json::json!({
                "contentItems": [{
                    "type": "inputText",
                    "text": format!("codexw rejected tool `{tool}`.")
                }],
                "failure_kind": "tool_request_rejected",
                "success": false
            }),
        },
    )?;
    Ok(true)
}
