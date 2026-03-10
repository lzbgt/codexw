use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;
use crate::background_shells::BackgroundShellManager;
use crate::local_api::LocalApiSnapshot;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;
use crate::local_api::snapshot::local_api_shell_job;

pub(in crate::local_api) fn json_request_body(
    request: &HttpRequest,
) -> std::result::Result<Value, HttpResponse> {
    serde_json::from_slice(&request.body)
        .map_err(|_| json_error_response(400, "invalid_json", "request body must be valid JSON"))
}

pub(in crate::local_api) fn resolve_shell_snapshot(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> std::result::Result<
    crate::local_api::snapshot::LocalApiBackgroundShellJob,
    (&'static str, &'static str),
> {
    if let Some(shell) = snapshot
        .workers
        .background_shells
        .iter()
        .find(|shell| shell.id == reference)
    {
        return Ok(shell.clone());
    }
    if let Some(shell) = snapshot
        .workers
        .background_shells
        .iter()
        .find(|shell| shell.alias.as_deref() == Some(reference))
    {
        return Ok(shell.clone());
    }
    if let Some(capability) = reference.strip_prefix('@') {
        let matches: Vec<_> = snapshot
            .capabilities
            .iter()
            .filter(|entry| entry.capability.trim_start_matches('@') == capability)
            .flat_map(|entry| entry.providers.iter())
            .filter_map(|provider| {
                snapshot
                    .workers
                    .background_shells
                    .iter()
                    .find(|shell| shell.id == provider.job_id)
                    .cloned()
            })
            .collect();
        return match matches.as_slice() {
            [shell] => Ok(shell.clone()),
            [] => Err(("shell_not_found", "unknown shell reference")),
            _ => Err(("shell_reference_ambiguous", "shell reference is ambiguous")),
        };
    }
    if let Ok(index) = reference.parse::<usize>() {
        if index == 0 {
            return Err(("validation_error", "shell index must be 1-based"));
        }
        if let Some(shell) = snapshot.workers.background_shells.get(index - 1) {
            return Ok(shell.clone());
        }
    }
    Err(("shell_not_found", "unknown shell reference"))
}

pub(in crate::local_api) fn current_shell_value(
    background_shells: &BackgroundShellManager,
    shell_id: &str,
) -> Option<Value> {
    background_shells
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.id == shell_id)
        .map(local_api_shell_job)
        .and_then(|shell| serde_json::to_value(shell).ok())
}

pub(in crate::local_api) fn session_payload(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    let session = session_summary(snapshot);
    json!({
        "ok": true,
        "session": session,
        "session_id": snapshot.session_id,
        "cwd": snapshot.cwd,
        "thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
        "objective": snapshot.objective,
        "working": snapshot.turn_running,
        "started_turn_count": snapshot.started_turn_count,
        "completed_turn_count": snapshot.completed_turn_count,
        "active_personality": snapshot.active_personality,
        "orchestration": snapshot.orchestration_status,
        "transcript_length": snapshot.transcript.len(),
    })
}

pub(in crate::local_api) fn attachment_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    let lease_active = snapshot
        .attachment_lease_expires_at_ms
        .is_some_and(|expiry| expiry > now_unix_ms());
    json!({
        "id": format!("attach:{}", snapshot.session_id),
        "scope": "process",
        "process_scoped": true,
        "client_id": snapshot.attachment_client_id,
        "lease_seconds": snapshot.attachment_lease_seconds,
        "lease_expires_at_ms": snapshot.attachment_lease_expires_at_ms,
        "lease_active": lease_active,
        "attached_thread_id": snapshot.thread_id,
    })
}

pub(in crate::local_api) fn session_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    json!({
        "id": snapshot.session_id,
        "scope": "process",
        "process_scoped": true,
        "attachment": attachment_summary(snapshot),
        "client_id": snapshot.attachment_client_id,
        "cwd": snapshot.cwd,
        "attached_thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
        "objective": snapshot.objective,
        "working": snapshot.turn_running,
        "started_turn_count": snapshot.started_turn_count,
        "completed_turn_count": snapshot.completed_turn_count,
        "active_personality": snapshot.active_personality,
        "transcript_length": snapshot.transcript.len(),
    })
}

pub(super) fn attachment_has_active_conflicting_client(
    snapshot: &LocalApiSnapshot,
    requested_client_id: Option<&str>,
) -> bool {
    let Some(existing_client_id) = snapshot.attachment_client_id.as_deref() else {
        return false;
    };
    if !attachment_lease_active(snapshot) {
        return false;
    }
    match requested_client_id {
        Some(requested_client_id) => existing_client_id != requested_client_id,
        None => true,
    }
}

fn attachment_lease_active(snapshot: &LocalApiSnapshot) -> bool {
    snapshot
        .attachment_lease_expires_at_ms
        .is_some_and(|expiry| expiry > now_unix_ms())
}

pub(in crate::local_api) fn parse_optional_client_id(
    body: &Value,
) -> Result<Option<String>, crate::local_api::server::HttpResponse> {
    let Some(value) = body.get("client_id") else {
        return Ok(None);
    };
    let Some(client_id) = value.as_str() else {
        return Err(json_error_response_with_details(
            400,
            "validation_error",
            "client_id must be a string",
            json!({
                "field": "client_id",
                "expected": "string",
            }),
        ));
    };
    let trimmed = client_id.trim();
    if trimmed.is_empty() {
        return Err(json_error_response_with_details(
            400,
            "validation_error",
            "client_id must not be empty",
            json!({
                "field": "client_id",
                "expected": "non-empty string",
            }),
        ));
    }
    Ok(Some(trimmed.to_string()))
}

pub(in crate::local_api) fn enforce_attachment_lease_ownership(
    snapshot: &LocalApiSnapshot,
    requested_client_id: Option<&str>,
) -> Result<(), crate::local_api::server::HttpResponse> {
    if attachment_has_active_conflicting_client(snapshot, requested_client_id) {
        return Err(json_error_response_with_details(
            409,
            "attachment_conflict",
            "another client currently holds the active attachment lease",
            json!({
                "session_id": snapshot.session_id,
                "requested_client_id": requested_client_id,
                "current_attachment": {
                    "client_id": snapshot.attachment_client_id,
                    "lease_seconds": snapshot.attachment_lease_seconds,
                    "lease_expires_at_ms": snapshot.attachment_lease_expires_at_ms,
                    "lease_active": attachment_lease_active(snapshot),
                }
            }),
        ));
    }
    Ok(())
}

pub(in crate::local_api) fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .ok()
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0)
}

pub(in crate::local_api) fn json_ok_response(body: serde_json::Value) -> HttpResponse {
    let body = match body {
        Value::Object(mut object) => {
            object.insert(
                "local_api_version".to_string(),
                Value::String(CODEXW_LOCAL_API_VERSION.to_string()),
            );
            Value::Object(object)
        }
        other => other,
    };
    HttpResponse {
        status: 200,
        reason: "OK",
        headers: vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string(),
        )],
        body: serde_json::to_vec_pretty(&body).unwrap_or_else(|_| b"{\"ok\":false}".to_vec()),
    }
}

pub(in crate::local_api) fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
) -> HttpResponse {
    json_error_response_with_details(status, code, message, json!({}))
}

pub(in crate::local_api) fn json_error_response_with_details(
    status: u16,
    code: &str,
    message: &str,
    details: serde_json::Value,
) -> HttpResponse {
    let reason = match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "Error",
    };
    json_ok_response(json!({
        "ok": false,
        "error": {
            "status": status,
            "code": code,
            "message": message,
            "retryable": status >= 500,
            "details": details,
        }
    }))
    .with_status(status, reason)
}

impl HttpResponse {
    pub(super) fn with_status(mut self, status: u16, reason: &'static str) -> Self {
        self.status = status;
        self.reason = reason;
        self
    }
}
