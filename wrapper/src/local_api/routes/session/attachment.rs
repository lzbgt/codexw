use serde_json::json;

use crate::local_api::LocalApiCommand;
use crate::local_api::SharedCommandQueue;
use crate::local_api::control::enqueue_command;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::super::attachment_summary;
use super::super::enforce_attachment_lease_ownership;
use super::super::json_error_response;
use super::super::json_ok_response;
use super::super::json_request_body;
use super::super::now_unix_ms;
use super::super::parse_optional_client_id;
use super::super::session_summary;
use super::optional_json_request_body;
use super::parse_optional_lease_seconds;

pub(super) fn handle_attachment_renew_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
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
) -> HttpResponse {
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
