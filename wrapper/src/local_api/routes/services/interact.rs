use serde_json::Value;
use serde_json::json;

use crate::background_shells::BackgroundShellManager;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::super::current_shell_value;
use super::super::enforce_attachment_lease_ownership;
use super::super::json_error_response;
use super::super::json_ok_response;
use super::super::json_request_body;
use super::super::parse_optional_client_id;
use super::super::resolve_shell_snapshot;

pub(super) fn handle_service_attach_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    background_shells: &BackgroundShellManager,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let body = if request.body.is_empty() {
        json!({})
    } else {
        match json_request_body(request) {
            Ok(value) => value,
            Err(response) => return response,
        }
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
                "requested_client_id": requested_client_id,
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
) -> HttpResponse {
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
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }
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
                "requested_client_id": requested_client_id,
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
) -> HttpResponse {
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
    let requested_client_id = match parse_optional_client_id(&body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
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
                "requested_client_id": requested_client_id,
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
