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

pub(super) fn handle_session_new_route(
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::StartSessionThread {
            session_id: snapshot.session_id.clone(),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue session creation: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
        "process_scoped": true,
        "requested_action": "start_thread",
    }))
}

pub(super) fn handle_session_attach_route(
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
    let Some(thread_id) = body.get("thread_id").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing thread_id");
    };
    if thread_id.trim().is_empty() {
        return json_error_response(400, "validation_error", "thread_id must not be empty");
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::AttachSessionThread {
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue session attach: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
        "target_thread_id": thread_id,
        "process_scoped": true,
        "requested_action": "attach_thread",
    }))
}
