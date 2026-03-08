use anyhow::Result;
use serde_json::json;
use std::process::ChildStdin;

use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::transcript_approval_summary::summarize_tool_request;
use crate::transcript_plan_render::build_tool_user_input_response;

pub(crate) fn handle_tool_request(
    request: &RpcRequest,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match request.method.as_str() {
        "tool/requestUserInput" | "item/tool/requestUserInput" => {
            let result = build_tool_user_input_response(&request.params);
            output.line_stderr(format!(
                "[input-request] auto-answered: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id.clone(),
                    result,
                },
            )?;
            Ok(true)
        }
        "mcpServer/elicitation/request" => {
            output.line_stderr(format!(
                "[input-request] auto-cancelled MCP elicitation: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id.clone(),
                    result: json!({"action": "cancel", "content": serde_json::Value::Null}),
                },
            )?;
            Ok(true)
        }
        "item/tool/call" => {
            output.line_stderr(format!(
                "[tool] unsupported dynamic tool call: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id.clone(),
                    result: json!({
                        "contentItems": [
                            {
                                "type": "inputText",
                                "text": "codexw does not implement dynamic tool calls"
                            }
                        ],
                        "success": false
                    }),
                },
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
