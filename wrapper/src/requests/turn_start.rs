use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::Cli;
use crate::collaboration_preset::current_collaboration_mode_value;
use crate::input::ParsedInput;
use crate::policy::approval_policy;
use crate::policy::reasoning_summary;
use crate::policy::turn_sandbox_policy;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_turn_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: String,
    submission: ParsedInput,
    auto_generated: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartTurn { auto_generated },
    );
    if !auto_generated && state.objective.is_none() && !submission.display_text.trim().is_empty() {
        state.objective = Some(submission.display_text.clone());
    }

    let mut params = json!({
        "threadId": thread_id,
        "input": submission.items,
        "cwd": resolved_cwd,
        "approvalPolicy": approval_policy(cli),
        "sandboxPolicy": turn_sandbox_policy(cli),
        "model": cli.model,
        "summary": reasoning_summary(cli),
    });
    if let Some(personality) = state.active_personality.as_deref() {
        params["personality"] = Value::String(personality.to_string());
    }
    if let Some(collaboration_mode) =
        current_collaboration_mode_value(state.active_collaboration_mode.as_ref())
    {
        params["collaborationMode"] = collaboration_mode;
    }

    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/start",
            params,
        },
    )
}

pub(crate) fn send_turn_steer(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    turn_id: String,
    submission: ParsedInput,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::SteerTurn {
            display_text: submission.display_text.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/steer",
            params: json!({
                "threadId": thread_id,
                "expectedTurnId": turn_id,
                "input": submission.items,
            }),
        },
    )
}
