#[path = "payload/json.rs"]
mod json;
#[path = "payload/wire.rs"]
mod wire;

use std::net::TcpStream;

use serde_json::Value;

use super::super::HttpResponse;

pub(super) fn write_response(
    stream: &mut TcpStream,
    response: &HttpResponse,
) -> anyhow::Result<()> {
    wire::write_response(stream, response)
}

pub(super) fn json_ok_response(body: Value) -> HttpResponse {
    json::json_ok_response(body)
}

pub(super) fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
    details: Option<Value>,
) -> HttpResponse {
    json::json_error_response(status, code, message, details)
}
