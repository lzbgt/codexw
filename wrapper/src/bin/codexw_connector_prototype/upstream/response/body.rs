use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;

use super::super::UpstreamResponse;
use super::head::read_upstream_head;

pub(super) fn read_upstream_response(mut stream: TcpStream) -> Result<UpstreamResponse> {
    let (status, reason, headers, remainder) = read_upstream_head(&mut stream)?;
    read_body(
        status,
        reason,
        headers,
        remainder,
        stream,
        "read upstream body",
    )
}

pub(super) fn read_error_body(
    status: u16,
    reason: String,
    headers: HashMap<String, String>,
    remainder: Vec<u8>,
    stream: TcpStream,
) -> Result<UpstreamResponse> {
    read_body(
        status,
        reason,
        headers,
        remainder,
        stream,
        "read upstream error body",
    )
}

fn read_body(
    status: u16,
    reason: String,
    headers: HashMap<String, String>,
    remainder: Vec<u8>,
    mut stream: TcpStream,
    context_label: &'static str,
) -> Result<UpstreamResponse> {
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(remainder.len());
    let mut body = remainder;
    let mut buffer = [0_u8; 4096];
    while body.len() < content_length {
        let read = stream.read(&mut buffer).context(context_label)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Ok(UpstreamResponse {
        status,
        reason,
        headers,
        body,
    })
}
