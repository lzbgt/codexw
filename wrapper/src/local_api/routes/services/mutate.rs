use serde_json::Value;
use serde_json::json;

use crate::local_api::LocalApiCommand;
use crate::local_api::SharedCommandQueue;
use crate::local_api::control::enqueue_command;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::super::enforce_attachment_lease_ownership;
use super::super::json_error_response;
use super::super::json_ok_response;
use super::super::json_request_body;
use super::super::parse_optional_client_id;
use super::super::resolve_shell_snapshot;

pub(super) fn handle_service_update_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("jobId") {
        return json_error_response(400, "validation_error", "missing jobId");
    }
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

pub(super) fn handle_dependency_update_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("jobId") {
        return json_error_response(400, "validation_error", "missing jobId");
    }
    if !object.contains_key("dependsOnCapabilities") {
        return json_error_response(400, "validation_error", "missing dependsOnCapabilities");
    }
    enqueue_dependency_update(command_queue, session_id, body, snapshot)
}

pub(super) fn handle_service_provide_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("capabilities") {
        return json_error_response(400, "validation_error", "missing capabilities");
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

pub(super) fn handle_service_depend_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("dependsOnCapabilities") {
        return json_error_response(400, "validation_error", "missing dependsOnCapabilities");
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_dependency_update(command_queue, session_id, body, snapshot)
}

pub(super) fn handle_service_contract_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    let has_contract_field = object.contains_key("protocol")
        || object.contains_key("endpoint")
        || object.contains_key("attachHint")
        || object.contains_key("readyPattern")
        || object.contains_key("recipes");
    if !has_contract_field {
        return json_error_response(
            400,
            "validation_error",
            "contract update requires one of protocol, endpoint, attachHint, readyPattern, or recipes",
        );
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

pub(super) fn handle_service_relabel_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("label") {
        return json_error_response(400, "validation_error", "missing label");
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

fn enqueue_service_update(
    command_queue: &SharedCommandQueue,
    session_id: &str,
    arguments: Value,
    snapshot: &LocalApiSnapshot,
) -> HttpResponse {
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::UpdateService {
            session_id: session_id.to_string(),
            arguments,
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue service update: {err:#}"),
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

fn enqueue_dependency_update(
    command_queue: &SharedCommandQueue,
    session_id: &str,
    arguments: Value,
    snapshot: &LocalApiSnapshot,
) -> HttpResponse {
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::UpdateDependencies {
            session_id: session_id.to_string(),
            arguments,
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue dependency update: {err:#}"),
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
