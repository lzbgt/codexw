use anyhow::Result;
use serde_json::Value;
use serde_json::json;
use std::process::ChildStdin;

use crate::Cli;
use crate::notifications::handle_notification;
use crate::output::Output;
use crate::policy::choose_command_approval_decision;
use crate::policy::choose_first_allowed_decision;
use crate::requests::send_json;
use crate::responses::handle_response;
use crate::rpc;
use crate::rpc::IncomingMessage;
use crate::rpc::OutgoingErrorObject;
use crate::rpc::OutgoingErrorResponse;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::runtime::StartMode;
use crate::state::AppState;
use crate::status_views::summarize_value;
use crate::transcript_render::build_tool_user_input_response;
use crate::transcript_summary::summarize_command_approval_request;
use crate::transcript_summary::summarize_generic_approval_request;
use crate::transcript_summary::summarize_tool_request;

pub(crate) fn process_server_line(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if state.raw_json {
        output.line_stderr(format!("[json] {line}"))?;
    }
    match rpc::parse_line(&line) {
        Ok(IncomingMessage::Response(response)) => handle_response(
            response,
            cli,
            resolved_cwd,
            state,
            output,
            writer,
            start_after_initialize,
        )?,
        Ok(IncomingMessage::Request(request)) => {
            handle_server_request(request, cli, output, writer)?;
        }
        Ok(IncomingMessage::Notification(notification)) => {
            handle_notification(notification, cli, resolved_cwd, state, output, writer)?;
        }
        Err(err) => {
            output.line_stderr(format!("[session] ignored malformed server line: {err}"))?;
        }
    }
    Ok(())
}

fn handle_server_request(
    request: RpcRequest,
    cli: &Cli,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    match request.method.as_str() {
        "item/commandExecution/requestApproval" => {
            let decision_value = choose_command_approval_decision(&request.params, cli.yolo);
            output.line_stderr(format!(
                "[approval] {}",
                summarize_command_approval_request(&request.params, &decision_value)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({"decision": decision_value}),
                },
            )?;
        }
        "item/fileChange/requestApproval" | "execCommandApproval" | "applyPatchApproval" => {
            let decision = params_auto_approval_result(&request.params);
            output.line_stderr(format!(
                "[approval] {}",
                summarize_generic_approval_request(&request.params, &request.method)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: decision,
                },
            )?;
        }
        "tool/requestUserInput" | "item/tool/requestUserInput" => {
            let result = build_tool_user_input_response(&request.params);
            output.line_stderr(format!(
                "[input-request] auto-answered: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result,
                },
            )?;
        }
        "mcpServer/elicitation/request" => {
            output.line_stderr(format!(
                "[input-request] auto-cancelled MCP elicitation: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({"action": "cancel", "content": Value::Null}),
                },
            )?;
        }
        "item/tool/call" => {
            output.line_stderr(format!(
                "[tool] unsupported dynamic tool call: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
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
        }
        _ => {
            if cli.verbose_events || cli.raw_json {
                output.line_stderr(format!(
                    "[server-request] {}: {}",
                    request.method,
                    if cli.raw_json {
                        serde_json::to_string_pretty(&request.params)?
                    } else {
                        summarize_value(&request.params)
                    }
                ))?;
            }
            send_json(
                writer,
                &OutgoingErrorResponse {
                    id: request.id,
                    error: OutgoingErrorObject {
                        code: -32601,
                        message: format!("codexw does not implement {}", request.method),
                        data: None,
                    },
                },
            )?;
        }
    }
    Ok(())
}

pub(crate) fn params_auto_approval_result(params: &Value) -> Value {
    if let Some(decisions) = params.get("availableDecisions").and_then(Value::as_array)
        && let Some(decision) = choose_first_allowed_decision(decisions)
    {
        return json!({"decision": decision});
    }
    json!({"decision": "accept"})
}
