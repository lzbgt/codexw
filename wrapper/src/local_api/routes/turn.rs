use serde_json::Value;
use serde_json::json;

use crate::local_api::LocalApiCommand;
use crate::local_api::SharedCommandQueue;
use crate::local_api::control::enqueue_command;
use crate::local_api::server::HttpRequest;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::json_error_response;
use super::json_ok_response;
use super::json_request_body;
use super::session_summary;

fn extract_prompt(body: &Value) -> Result<&str, crate::local_api::server::HttpResponse> {
    let Some(prompt) = body
        .get("input")
        .and_then(Value::as_object)
        .and_then(|input| input.get("text"))
        .and_then(Value::as_str)
    else {
        return Err(json_error_response(
            400,
            "validation_error",
            "missing input.text",
        ));
    };
    if prompt.trim().is_empty() {
        return Err(json_error_response(
            400,
            "validation_error",
            "input.text must not be empty",
        ));
    }
    Ok(prompt)
}

pub(super) fn handle_turn_start_route(
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
    handle_turn_start_for_session(&session_id, body, snapshot, command_queue)
}

pub(super) fn handle_turn_start_route_for_session(
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
    handle_turn_start_for_session(&snapshot.session_id, body, snapshot, command_queue)
}

fn handle_turn_start_for_session(
    session_id: &str,
    body: Value,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    if snapshot.thread_id.is_none() {
        return json_error_response(409, "thread_not_attached", "session has no attached thread");
    }
    let prompt = match extract_prompt(&body) {
        Ok(prompt) => prompt,
        Err(response) => return response,
    };
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::StartTurn {
            session_id: session_id.to_string(),
            prompt: prompt.to_string(),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue start request: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "session": session_summary(snapshot),
        "operation": {
            "kind": "turn.start",
            "queued": true,
        },
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
    }))
}

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
    handle_turn_interrupt_for_session(&session_id, snapshot, command_queue)
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
    handle_turn_interrupt_for_session(&snapshot.session_id, snapshot, command_queue)
}

fn handle_turn_interrupt_for_session(
    session_id: &str,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
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
        },
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
}
