use serde_json::Value;
use serde_json::json;

use crate::local_api::LocalApiCommand;
use crate::local_api::SharedCommandQueue;
use crate::local_api::control::enqueue_command;
use crate::local_api::server::HttpRequest;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::attachment_summary;
use super::enforce_attachment_lease_ownership;
use super::json_error_response;
use super::json_ok_response;
use super::json_request_body;
use super::now_unix_ms;
use super::parse_optional_client_id;
use super::session_summary;

pub(super) fn handle_session_new_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    let body = match optional_json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let lease_seconds = match parse_optional_lease_seconds(&body) {
        Ok(value) => value,
        Err(message) => return json_error_response(400, "validation_error", message),
    };
    if let Err(response) = enforce_attachment_lease_ownership(snapshot, client_id.as_deref()) {
        return response;
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::StartSessionThread {
            session_id: snapshot.session_id.clone(),
            client_id: client_id.clone(),
            lease_seconds,
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
        "session": session_summary(snapshot),
        "attachment": attachment_summary(snapshot),
        "operation": {
            "kind": "session.new",
            "queued": true,
            "requested_action": "start_thread",
            "requested_client_id": client_id,
            "requested_lease_seconds": lease_seconds,
            "requested_lease_expires_at_ms": lease_seconds.map(|seconds| now_unix_ms() + (seconds * 1000)),
        },
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
    let client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let lease_seconds = match parse_optional_lease_seconds(&body) {
        Ok(value) => value,
        Err(message) => return json_error_response(400, "validation_error", message),
    };
    if let Err(response) = enforce_attachment_lease_ownership(snapshot, client_id.as_deref()) {
        return response;
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::AttachSessionThread {
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
            client_id: client_id.clone(),
            lease_seconds,
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
        "session": session_summary(snapshot),
        "attachment": attachment_summary(snapshot),
        "operation": {
            "kind": "session.attach",
            "queued": true,
            "requested_action": "attach_thread",
            "target_thread_id": thread_id,
            "requested_client_id": client_id,
            "requested_lease_seconds": lease_seconds,
            "requested_lease_expires_at_ms": lease_seconds.map(|seconds| now_unix_ms() + (seconds * 1000)),
        },
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
        "target_thread_id": thread_id,
        "process_scoped": true,
        "requested_action": "attach_thread",
    }))
}

pub(super) fn handle_attachment_renew_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(lease_seconds) = (match parse_optional_lease_seconds(&body) {
        Ok(value) => value,
        Err(message) => return json_error_response(400, "validation_error", message),
    }) else {
        return json_error_response(400, "validation_error", "missing lease_seconds");
    };
    if let Err(response) = enforce_attachment_lease_ownership(snapshot, client_id.as_deref()) {
        return response;
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::RenewAttachmentLease {
            session_id: snapshot.session_id.clone(),
            client_id: client_id.clone(),
            lease_seconds,
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue attachment renewal: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "session": session_summary(snapshot),
        "attachment": attachment_summary(snapshot),
        "operation": {
            "kind": "attachment.renew",
            "queued": true,
            "requested_client_id": client_id,
            "requested_lease_seconds": lease_seconds,
            "requested_lease_expires_at_ms": now_unix_ms() + (lease_seconds * 1000),
        },
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
    }))
}

pub(super) fn handle_attachment_release_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    let body = match optional_json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) = enforce_attachment_lease_ownership(snapshot, client_id.as_deref()) {
        return response;
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::ReleaseAttachment {
            session_id: snapshot.session_id.clone(),
            client_id: client_id.clone(),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue attachment release: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "session": session_summary(snapshot),
        "attachment": attachment_summary(snapshot),
        "operation": {
            "kind": "attachment.release",
            "queued": true,
            "requested_client_id": client_id,
        },
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
    }))
}

fn optional_json_request_body(
    request: &HttpRequest,
) -> Result<Value, crate::local_api::server::HttpResponse> {
    if request.body.is_empty() {
        Ok(json!({}))
    } else {
        json_request_body(request)
    }
}

fn parse_optional_lease_seconds(body: &Value) -> Result<Option<u64>, &'static str> {
    let Some(value) = body.get("lease_seconds") else {
        return Ok(None);
    };
    let Some(lease_seconds) = value.as_u64() else {
        return Err("lease_seconds must be a positive integer");
    };
    if lease_seconds == 0 {
        return Err("lease_seconds must be greater than zero");
    }
    Ok(Some(lease_seconds))
}
