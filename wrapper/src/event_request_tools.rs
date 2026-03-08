use anyhow::Result;
use std::process::ChildStdin;

use crate::client_dynamic_tools::execute_dynamic_tool_call;
use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::state::AppState;
use crate::transcript_approval_summary::summarize_tool_request;
use crate::transcript_plan_render::build_mcp_elicitation_response;
use crate::transcript_plan_render::build_tool_user_input_response;

pub(crate) fn handle_tool_request(
    request: &RpcRequest,
    resolved_cwd: &str,
    state: &mut AppState,
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
            let result = build_mcp_elicitation_response(&request.params);
            let action = result
                .get("action")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("cancel");
            output.line_stderr(format!(
                "[input-request] auto-{action} MCP elicitation: {}",
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
        "item/tool/call" => {
            let result = execute_dynamic_tool_call(
                &request.params,
                resolved_cwd,
                &state.orchestration.background_shells,
            );
            let success = result
                .get("success")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
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
            Ok(true)
        }
        _ => Ok(false),
    }
}
