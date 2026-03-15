use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;

use super::super::MAX_REQUEST_BYTES;
use super::UpstreamResponse;

pub(super) fn read_upstream_response(mut stream: TcpStream) -> Result<UpstreamResponse> {
    let (status, reason, headers, remainder) = read_upstream_head(&mut stream)?;
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = remainder;
    let mut buffer = [0_u8; 4096];
    while body.len() < content_length {
        let read = stream.read(&mut buffer).context("read upstream body")?;
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

pub(super) fn read_error_body(
    status: u16,
    reason: String,
    headers: HashMap<String, String>,
    remainder: Vec<u8>,
    mut stream: TcpStream,
) -> Result<UpstreamResponse> {
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(remainder.len());
    let mut body = remainder;
    let mut buffer = [0_u8; 4096];
    while body.len() < content_length {
        let read = stream
            .read(&mut buffer)
            .context("read upstream error body")?;
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

pub(super) fn read_upstream_head(
    stream: &mut TcpStream,
) -> Result<(u16, String, HashMap<String, String>, Vec<u8>)> {
    let mut buffer = [0_u8; 1024];
    let mut response_bytes = Vec::new();
    let header_end = loop {
        let read = stream.read(&mut buffer).context("read upstream response")?;
        if read == 0 {
            anyhow::bail!("upstream closed before headers");
        }
        response_bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = response_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            break index + 4;
        }
        if response_bytes.len() >= MAX_REQUEST_BYTES {
            anyhow::bail!("upstream response headers too large");
        }
    };
    let header_text = String::from_utf8_lossy(&response_bytes[..header_end]);
    let mut lines = header_text.split("\r\n");
    let status_line = lines.next().context("missing upstream status line")?;
    let mut status_parts = status_line.splitn(3, ' ');
    let _http_version = status_parts.next().context("missing upstream version")?;
    let status = status_parts
        .next()
        .context("missing upstream status code")?
        .parse::<u16>()
        .context("parse upstream status code")?;
    let reason = status_parts.next().unwrap_or("").to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok((
        status,
        reason,
        headers,
        response_bytes[header_end..].to_vec(),
    ))
}
