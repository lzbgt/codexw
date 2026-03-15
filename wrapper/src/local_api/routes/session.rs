#[path = "session/attachment.rs"]
mod attachment;
#[path = "session/lifecycle.rs"]
mod lifecycle;

use serde_json::Value;

use crate::local_api::SharedCommandQueue;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;
use crate::local_api::snapshot::LocalApiSnapshot;

pub(super) fn handle_session_new_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    lifecycle::handle_session_new_route(request, snapshot, command_queue)
}

pub(super) fn handle_session_attach_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    lifecycle::handle_session_attach_route(request, snapshot, command_queue)
}

pub(super) fn handle_attachment_renew_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    attachment::handle_attachment_renew_route(request, snapshot, command_queue)
}

pub(super) fn handle_attachment_release_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    attachment::handle_attachment_release_route(request, snapshot, command_queue)
}

fn optional_json_request_body(
    request: &HttpRequest,
) -> Result<Value, crate::local_api::server::HttpResponse> {
    if request.body.is_empty() {
        Ok(serde_json::json!({}))
    } else {
        super::json_request_body(request)
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
