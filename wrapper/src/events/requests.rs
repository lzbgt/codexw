use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;

use crate::Cli;
use crate::event_request_approvals::handle_approval_request;
use crate::event_request_tools::handle_tool_request;
use crate::output::Output;
use crate::rpc;
use crate::runtime_event_sources::AppEvent;
use crate::state::AppState;

pub(crate) fn handle_server_request(
    request: rpc::RpcRequest,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<()> {
    if handle_approval_request(&request, cli, output, writer)? {
        return Ok(());
    }
    if handle_tool_request(&request, resolved_cwd, state, output, writer, tx)? {
        return Ok(());
    }
    if cli.verbose_events || cli.raw_json {
        output.line_stderr(format!(
            "[server-request] {}: {}",
            request.method,
            if cli.raw_json {
                serde_json::to_string_pretty(&request.params)?
            } else {
                crate::status_value::summarize_value(&request.params)
            }
        ))?;
    }
    crate::requests::send_json(
        writer,
        &rpc::OutgoingErrorResponse {
            id: request.id,
            error: rpc::OutgoingErrorObject {
                code: -32601,
                message: format!("codexw does not implement {}", request.method),
                data: None,
            },
        },
    )?;
    Ok(())
}
