use serde_json::Value;
use serde_json::json;

use crate::local_api::LocalApiSnapshot;
use crate::local_api::SharedEventLog;
use crate::local_api::publish_client_event;

use super::enforce_attachment_lease_ownership;
use super::json_error_response_with_details;
use super::json_ok_response;
use super::json_request_body;
use super::parse_optional_client_id;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;

pub(super) fn handle_client_event_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    event_log: &SharedEventLog,
) -> HttpResponse {
    let body = match json_request_body(request) {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some(session_id) = body.get("session_id").and_then(Value::as_str) else {
        return json_error_response_with_details(
            400,
            "validation_error",
            "session_id must be a string",
            json!({
                "field": "session_id",
                "expected": "string",
            }),
        );
    };
    if session_id != snapshot.session_id {
        return super::json_error_response(404, "session_not_found", "unknown session id");
    }
    handle_client_event_body(request, snapshot, event_log, session_id, &body)
}

pub(super) fn handle_session_client_event_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    event_log: &SharedEventLog,
    session_id: &str,
) -> HttpResponse {
    let body = match json_request_body(request) {
        Ok(body) => body,
        Err(response) => return response,
    };
    handle_client_event_body(request, snapshot, event_log, session_id, &body)
}

fn handle_client_event_body(
    _request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    event_log: &SharedEventLog,
    session_id: &str,
    body: &Value,
) -> HttpResponse {
    let requested_client_id = match parse_optional_client_id(body) {
        Ok(client_id) => client_id,
        Err(response) => return response,
    };
    if let Err(response) =
        enforce_attachment_lease_ownership(snapshot, requested_client_id.as_deref())
    {
        return response;
    }

    let Some(event_name) = body.get("event").and_then(Value::as_str) else {
        return json_error_response_with_details(
            400,
            "validation_error",
            "event must be a string",
            json!({
                "field": "event",
                "expected": "string",
            }),
        );
    };
    let event_name = event_name.trim();
    if event_name.is_empty() {
        return json_error_response_with_details(
            400,
            "validation_error",
            "event must not be empty",
            json!({
                "field": "event",
                "expected": "non-empty string",
            }),
        );
    }

    let event_data = body.get("data").cloned().unwrap_or_else(|| json!({}));
    publish_client_event(
        event_log,
        session_id,
        requested_client_id.as_deref(),
        event_name,
        event_data.clone(),
    );

    json_ok_response(json!({
        "ok": true,
        "session_id": session_id,
        "client_id": requested_client_id,
        "event": event_name,
        "data": event_data,
        "operation": {
            "kind": "client.event",
        }
    }))
}
