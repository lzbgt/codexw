#[path = "shared/attachment.rs"]
mod attachment;
#[path = "shared/payload.rs"]
mod payload;
#[path = "shared/request.rs"]
mod request;
#[path = "shared/response.rs"]
mod response;
#[path = "shared/shell_refs.rs"]
mod shell_refs;

use anyhow::Result;
use serde_json::Value;

use crate::background_shells::BackgroundShellManager;
use crate::local_api::LocalApiSnapshot;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;

pub(in crate::local_api) fn json_request_body(
    request: &HttpRequest,
) -> std::result::Result<Value, HttpResponse> {
    request::json_request_body(request)
}

pub(in crate::local_api) fn resolve_shell_snapshot(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> std::result::Result<
    crate::local_api::snapshot::LocalApiBackgroundShellJob,
    (&'static str, &'static str),
> {
    shell_refs::resolve_shell_snapshot(snapshot, reference)
}

pub(in crate::local_api) fn current_shell_value(
    background_shells: &BackgroundShellManager,
    shell_id: &str,
) -> Option<Value> {
    shell_refs::current_shell_value(background_shells, shell_id)
}

pub(in crate::local_api) fn session_payload(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    payload::session_payload(snapshot)
}

pub(in crate::local_api) fn attachment_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    payload::attachment_summary(snapshot)
}

pub(in crate::local_api) fn session_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    payload::session_summary(snapshot)
}

pub(in crate::local_api) fn parse_optional_client_id(
    body: &Value,
) -> Result<Option<String>, crate::local_api::server::HttpResponse> {
    request::parse_optional_client_id(body)
}

pub(in crate::local_api) fn enforce_attachment_lease_ownership(
    snapshot: &LocalApiSnapshot,
    requested_client_id: Option<&str>,
) -> Result<(), crate::local_api::server::HttpResponse> {
    attachment::enforce_attachment_lease_ownership(snapshot, requested_client_id)
}

pub(in crate::local_api) fn now_unix_ms() -> u64 {
    attachment::now_unix_ms()
}

pub(in crate::local_api) fn json_ok_response(body: serde_json::Value) -> HttpResponse {
    response::json_ok_response(body)
}

pub(in crate::local_api) fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
) -> HttpResponse {
    response::json_error_response(status, code, message)
}

pub(in crate::local_api) fn json_error_response_with_details(
    status: u16,
    code: &str,
    message: &str,
    details: serde_json::Value,
) -> HttpResponse {
    response::json_error_response_with_details(status, code, message, details)
}
