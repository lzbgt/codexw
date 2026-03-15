#[path = "response/adapt.rs"]
mod adapt;
#[path = "response/payload.rs"]
mod payload;

use std::net::TcpStream;

use serde_json::Value;

use super::super::Cli;
use super::super::upstream::UpstreamResponse;
use super::HttpResponse;

pub(super) fn from_upstream_response(upstream: UpstreamResponse, cli: &Cli) -> HttpResponse {
    adapt::from_upstream_response(upstream, cli)
}

pub(super) fn write_response(
    stream: &mut TcpStream,
    response: &HttpResponse,
) -> anyhow::Result<()> {
    payload::write_response(stream, response)
}

pub(super) fn json_ok_response(body: Value) -> HttpResponse {
    payload::json_ok_response(body)
}

pub(super) fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
    details: Option<Value>,
) -> HttpResponse {
    payload::json_error_response(status, code, message, details)
}
