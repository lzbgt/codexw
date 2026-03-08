use anyhow::Result;
use std::process::ChildStdin;

use crate::Cli;
use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingErrorObject;
use crate::rpc::OutgoingErrorResponse;
use crate::rpc::RpcRequest;
use crate::status_views::summarize_value;

#[path = "event_request_approvals.rs"]
mod event_request_approvals;
#[path = "event_request_tools.rs"]
mod event_request_tools;

use event_request_approvals::handle_approval_request;
pub(crate) use event_request_approvals::params_auto_approval_result;
use event_request_tools::handle_tool_request;

pub(crate) fn handle_server_request(
    request: RpcRequest,
    cli: &Cli,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    if handle_approval_request(&request, cli, output, writer)? {
        return Ok(());
    }
    if handle_tool_request(&request, output, writer)? {
        return Ok(());
    }
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
    Ok(())
}
