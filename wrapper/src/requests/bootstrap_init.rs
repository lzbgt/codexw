use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::Cli;
use crate::rpc::OutgoingNotification;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_initialize(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    experimental_api: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::Initialize);
    let mut capabilities = json!({
        "experimentalApi": experimental_api,
    });
    if !cli.raw_json {
        capabilities["optOutNotificationMethods"] = json!([
            "item/agentMessage/delta",
            "item/reasoning/summaryTextDelta",
            "item/reasoning/summaryPartAdded",
            "item/reasoning/textDelta",
            "item/plan/delta"
        ]);
    }
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "initialize",
            params: json!({
                "clientInfo": {
                    "name": "codexw_terminal",
                    "title": "CodexW Terminal",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": capabilities
            }),
        },
    )
}

pub(crate) fn send_initialized(writer: &mut ChildStdin) -> Result<()> {
    send_json(
        writer,
        &OutgoingNotification {
            method: "initialized",
            params: Value::Null,
        },
    )
}
