#[path = "response/body.rs"]
mod body;
#[path = "response/head.rs"]
mod head;

use std::collections::HashMap;
use std::net::TcpStream;

use anyhow::Result;

use super::UpstreamResponse;

pub(super) fn read_upstream_response(stream: TcpStream) -> Result<UpstreamResponse> {
    body::read_upstream_response(stream)
}

pub(super) fn read_error_body(
    status: u16,
    reason: String,
    headers: HashMap<String, String>,
    remainder: Vec<u8>,
    stream: TcpStream,
) -> Result<UpstreamResponse> {
    body::read_error_body(status, reason, headers, remainder, stream)
}

pub(super) fn read_upstream_head(
    stream: &mut TcpStream,
) -> Result<(u16, String, HashMap<String, String>, Vec<u8>)> {
    head::read_upstream_head(stream)
}
