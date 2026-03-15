use serde_json::Value;
use serde_json::json;

use crate::local_api::LocalApiCommand;
use crate::local_api::SharedCommandQueue;
use crate::local_api::control::enqueue_command;
use crate::local_api::server::HttpRequest;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::super::enforce_attachment_lease_ownership;
use super::super::json_error_response;
use super::super::json_ok_response;
use super::super::json_request_body;
use super::super::parse_optional_client_id;
use super::super::session_summary;

pub(super) fn handle_turn_interrupt_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(session_id) = body.get("session_id").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing session_id");
    };
    if session_id != snapshot.session_id {
        return json_error_response(404, "session_not_found", "unknown session id");
    }
    let session_id = session_id.to_string();
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    handle_turn_interrupt_for_session(&session_id, requested_client_id, snapshot, command_queue)
}

pub(super) fn handle_turn_interrupt_route_for_session(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Some(body_session_id) = body.get("session_id").and_then(Value::as_str) {
        if body_session_id != snapshot.session_id {
            return json_error_response(404, "session_not_found", "unknown session id");
        }
    }
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    handle_turn_interrupt_for_session(
        &snapshot.session_id,
        requested_client_id,
        snapshot,
        command_queue,
    )
}

fn handle_turn_interrupt_for_session(
    session_id: &str,
    requested_client_id: Option<String>,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::InterruptTurn {
            session_id: session_id.to_string(),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue interrupt request: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "session": session_summary(snapshot),
        "operation": {
            "kind": "turn.interrupt",
            "queued": true,
            "requested_client_id": requested_client_id,
        },
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
}
