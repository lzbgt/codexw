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
use super::resolve_shell_snapshot;

pub(super) fn handle_shells_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "shells": snapshot.workers.background_shells,
    }))
}

pub(super) fn handle_shell_start_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> crate::local_api::server::HttpResponse {
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("command") {
        return json_error_response(400, "validation_error", "missing command");
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::StartShell {
            session_id: snapshot.session_id.clone(),
            arguments: body,
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue shell start: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
}

pub(super) fn handle_shell_poll_route(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "shell": shell,
        })),
        Err((code, message)) => json_error_response(404, code, message),
    }
}

pub(super) fn handle_shell_send_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    let Some(text) = object.get("text").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing text");
    };
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::SendShellInput {
            session_id: snapshot.session_id.clone(),
            arguments: json!({
                "jobId": shell.id,
                "text": text,
                "appendNewline": object.get("appendNewline").cloned().unwrap_or_else(|| Value::Bool(true)),
            }),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue shell input: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
}

pub(super) fn handle_shell_terminate_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    if !request.body.is_empty() {
        if let Err(response) = json_request_body(request) {
            return response;
        }
    }
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::TerminateShell {
            session_id: snapshot.session_id.clone(),
            arguments: json!({ "jobId": shell.id }),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue shell termination: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
}
