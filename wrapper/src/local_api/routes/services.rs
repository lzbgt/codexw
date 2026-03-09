use serde_json::Value;
use serde_json::json;

use crate::background_shells::BackgroundShellManager;
use crate::local_api::LocalApiCommand;
use crate::local_api::SharedCommandQueue;
use crate::local_api::control::enqueue_command;
use crate::local_api::server::HttpRequest;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::current_shell_value;
use super::json_error_response;
use super::json_ok_response;
use super::json_request_body;
use super::resolve_shell_snapshot;

pub(super) fn handle_services_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    let services: Vec<_> = snapshot
        .workers
        .background_shells
        .iter()
        .filter(|shell| shell.intent == "service")
        .cloned()
        .collect();
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "services": services,
    }))
}

pub(super) fn handle_capabilities_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "capabilities": snapshot.capabilities,
    }))
}

pub(super) fn handle_service_attach_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    background_shells: &BackgroundShellManager,
    reference: &str,
    session_id: &str,
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
    match background_shells.attach_from_tool(&json!({ "jobId": shell.id })) {
        Ok(attachment) => json_ok_response(json!({
            "ok": true,
            "session_id": session_id,
            "shell_id": shell.id,
            "service": current_shell_value(background_shells, &shell.id)
                .unwrap_or_else(|| json!(shell.clone())),
            "interaction": {
                "kind": "attach",
                "reference": reference,
            },
            "attachment": attachment,
            "attachment_text": attachment,
        })),
        Err(err) => json_error_response(400, "interaction_error", &err),
    }
}

pub(super) fn handle_service_wait_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    background_shells: &BackgroundShellManager,
    reference: &str,
    session_id: &str,
) -> crate::local_api::server::HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let body = if request.body.is_empty() {
        json!({})
    } else {
        match json_request_body(request) {
            Ok(value) => value,
            Err(response) => return response,
        }
    };
    let Some(object) = body.as_object() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    let mut arguments = serde_json::Map::new();
    arguments.insert("jobId".to_string(), Value::String(shell.id.clone()));
    let timeout_ms = object
        .get("timeoutMs")
        .and_then(Value::as_u64)
        .unwrap_or(crate::background_shells::DEFAULT_READY_WAIT_TIMEOUT_MS);
    if let Some(timeout_ms) = object.get("timeoutMs") {
        arguments.insert("timeoutMs".to_string(), timeout_ms.clone());
    }
    match background_shells.wait_ready_from_tool(&Value::Object(arguments)) {
        Ok(result) => json_ok_response(json!({
            "ok": true,
            "session_id": session_id,
            "shell_id": shell.id,
            "service": current_shell_value(background_shells, &shell.id)
                .unwrap_or_else(|| json!(shell.clone())),
            "interaction": {
                "kind": "wait",
                "reference": reference,
                "timeout_ms": timeout_ms,
            },
            "result": result,
            "result_text": result,
        })),
        Err(err) => json_error_response(400, "interaction_error", &err),
    }
}

pub(super) fn handle_service_run_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    background_shells: &BackgroundShellManager,
    reference: &str,
    session_id: &str,
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
    let Some(recipe) = object.get("recipe").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing recipe");
    };
    if recipe.trim().is_empty() {
        return json_error_response(400, "validation_error", "recipe must not be empty");
    }
    let mut arguments = serde_json::Map::new();
    arguments.insert("jobId".to_string(), Value::String(shell.id.clone()));
    arguments.insert(
        "recipe".to_string(),
        Value::String(recipe.trim().to_string()),
    );
    if let Some(args) = object.get("args") {
        arguments.insert("args".to_string(), args.clone());
    }
    if let Some(wait_for_ready_ms) = object.get("waitForReadyMs") {
        arguments.insert("waitForReadyMs".to_string(), wait_for_ready_ms.clone());
    }
    let args_value = object.get("args").cloned();
    let wait_for_ready_ms = object.get("waitForReadyMs").and_then(Value::as_u64);
    match background_shells.invoke_recipe_from_tool(&Value::Object(arguments)) {
        Ok(result) => json_ok_response(json!({
            "ok": true,
            "session_id": session_id,
            "shell_id": shell.id,
            "service": current_shell_value(background_shells, &shell.id)
                .unwrap_or_else(|| json!(shell.clone())),
            "interaction": {
                "kind": "run",
                "reference": reference,
            },
            "recipe": {
                "name": recipe.trim(),
                "args": args_value,
                "wait_for_ready_ms": wait_for_ready_ms,
            },
            "result": result,
            "result_text": result,
        })),
        Err(err) => json_error_response(400, "interaction_error", &err),
    }
}

pub(super) fn handle_service_update_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> crate::local_api::server::HttpResponse {
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
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
) -> crate::local_api::server::HttpResponse {
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
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
) -> crate::local_api::server::HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
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
) -> crate::local_api::server::HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
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
) -> crate::local_api::server::HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
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
) -> crate::local_api::server::HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
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
) -> crate::local_api::server::HttpResponse {
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
) -> crate::local_api::server::HttpResponse {
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
