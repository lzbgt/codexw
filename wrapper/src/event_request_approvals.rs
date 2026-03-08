use anyhow::Result;
use serde_json::Value;
use serde_json::json;
use std::process::ChildStdin;

use crate::Cli;
use crate::output::Output;
use crate::policy::choose_command_approval_decision;
use crate::policy::choose_first_allowed_decision;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::transcript_approval_summary::summarize_command_approval_request;
use crate::transcript_approval_summary::summarize_generic_approval_request;

pub(crate) fn handle_approval_request(
    request: &RpcRequest,
    cli: &Cli,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
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
                    id: request.id.clone(),
                    result: json!({"decision": decision_value}),
                },
            )?;
            Ok(true)
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
                    id: request.id.clone(),
                    result: decision,
                },
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(crate) fn params_auto_approval_result(params: &Value) -> Value {
    if let Some(decisions) = params.get("availableDecisions").and_then(Value::as_array)
        && let Some(decision) = choose_first_allowed_decision(decisions)
    {
        return json!({"decision": decision});
    }
    json!({"decision": "accept"})
}
