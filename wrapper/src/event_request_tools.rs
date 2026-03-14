use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;
use std::thread;

use crate::client_dynamic_tools::execute_background_shell_tool_call_with_manager;
use crate::client_dynamic_tools::execute_dynamic_tool_call_with_state;
use crate::client_dynamic_tools::is_background_shell_tool;
use crate::client_dynamic_tools::legacy_workspace_tool_failure_notice;
use crate::client_dynamic_tools::legacy_workspace_tool_notice;
use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_event_sources::AsyncToolResponse;
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
    tx: &mpsc::Sender<AppEvent>,
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
            let tool = request
                .params
                .get("tool")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("dynamic tool");
            if is_background_shell_tool(tool) {
                let request_id = request.id.clone();
                let params = request.params.clone();
                let summary = summarize_tool_request(&params);
                let tool_name = tool.to_string();
                let resolved_cwd = resolved_cwd.to_string();
                let tx = tx.clone();
                let background_shells = state.orchestration.background_shells.clone();
                thread::spawn(move || {
                    let result = execute_background_shell_tool_call_with_manager(
                        &params,
                        &resolved_cwd,
                        &background_shells,
                    );
                    let _ = tx.send(AppEvent::AsyncToolResponseReady(AsyncToolResponse {
                        id: request_id,
                        tool: tool_name,
                        summary,
                        result,
                    }));
                });
                return Ok(true);
            }
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
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::RequestId;
    use crate::runtime_event_sources::AsyncToolResponse;
    use serde_json::json;
    use std::process::Command;
    use std::process::Stdio;
    use std::time::Duration;

    fn spawn_sink_stdin() -> std::process::ChildStdin {
        Command::new("sh")
            .arg("-c")
            .arg("cat >/dev/null")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sink")
            .stdin
            .take()
            .expect("stdin")
    }

    fn test_request(method: &str, tool: &str, arguments: serde_json::Value) -> RpcRequest {
        RpcRequest {
            id: RequestId::Integer(7),
            method: method.to_string(),
            params: json!({
                "tool": tool,
                "threadId": "thread-1",
                "callId": "call-1",
                "arguments": arguments,
            }),
        }
    }

    #[test]
    fn background_shell_tool_requests_complete_asynchronously() {
        let mut state = AppState::new(true, false);
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();
        let (tx, rx) = mpsc::channel();
        let request = test_request(
            "item/tool/call",
            "background_shell_start",
            json!({"command": "printf 'alpha\\n'"}),
        );

        let handled =
            handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
                .expect("handle tool request");

        assert!(handled);
        let event = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("async tool response");
        let AsyncToolResponse {
            id,
            tool,
            summary,
            result,
        } = match event {
            AppEvent::AsyncToolResponseReady(event) => event,
            other => panic!("expected async tool response, got {other:?}"),
        };
        assert_eq!(id, RequestId::Integer(7));
        assert_eq!(tool, "background_shell_start");
        assert!(summary.contains("background_shell_start"));
        assert_eq!(result["success"], true);
    }

    #[test]
    fn non_shell_dynamic_tool_requests_do_not_enqueue_async_response() {
        let mut state = AppState::new(true, false);
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();
        let (tx, rx) = mpsc::channel();
        let request = test_request("item/tool/call", "orchestration_status", json!({}));

        let handled =
            handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
                .expect("handle tool request");

        assert!(handled);
        assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());
    }
}
