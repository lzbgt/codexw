use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use url::Url;

use super::Cli;
use super::MAX_REQUEST_BYTES;
use super::http::HttpRequest;
use super::routing::ProxyTarget;

#[derive(Debug, Clone)]
pub(super) struct UpstreamResponse {
    pub(super) status: u16,
    pub(super) reason: String,
    pub(super) headers: HashMap<String, String>,
    pub(super) body: Vec<u8>,
}

pub(super) fn forward_request(
    request: &HttpRequest,
    cli: &Cli,
    target: &ProxyTarget,
) -> Result<UpstreamResponse> {
    let base = Url::parse(&cli.local_api_base).context("parse local API base URL")?;
    let host = base
        .host_str()
        .context("local API base URL missing host")?
        .to_string();
    let port = base
        .port_or_known_default()
        .context("local API base URL missing port")?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("connect to local API {}:{}", host, port))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .context("set upstream read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("set upstream write timeout")?;

    let path = compose_local_path(&base, &target.local_path);
    let (content_type, body) = prepare_upstream_body(request, target)?;
    write_upstream_request(
        &mut stream,
        &request.method,
        &path,
        content_type.as_deref(),
        cli.local_api_token.as_deref(),
        body.as_slice(),
        request.headers.get("last-event-id").map(String::as_str),
    )?;

    read_upstream_response(stream)
}

pub(super) fn prepare_upstream_body(
    request: &HttpRequest,
    target: &ProxyTarget,
) -> Result<(Option<String>, Vec<u8>)> {
    let requested_client_id = request
        .headers
        .get("x-codexw-client-id")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let requested_lease_seconds = request
        .headers
        .get("x-codexw-lease-seconds")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    let requires_object_body =
        target.session_id_hint.is_some() || supports_client_lease_injection(&target.local_path);
    if request.method != "POST" || !requires_object_body {
        return Ok((
            request.headers.get("content-type").cloned(),
            request.body.clone(),
        ));
    }

    if requested_client_id.is_none()
        && requested_lease_seconds.is_none()
        && target.session_id_hint.is_none()
    {
        return Ok((
            request.headers.get("content-type").cloned(),
            request.body.clone(),
        ));
    }

    let mut object = if request.body.is_empty() {
        serde_json::Map::new()
    } else {
        let value: Value = serde_json::from_slice(&request.body)
            .context("parse connector request body for connector JSON injection")?;
        let Some(object) = value.as_object() else {
            anyhow::bail!("connector JSON injection requires a JSON object body");
        };
        object.clone()
    };

    if let Some(session_id) = &target.session_id_hint {
        object
            .entry("session_id".to_string())
            .or_insert(Value::String(session_id.clone()));
    }

    if let Some(client_id) = requested_client_id {
        object
            .entry("client_id".to_string())
            .or_insert(Value::String(client_id));
    }
    if let Some(lease_seconds) = requested_lease_seconds {
        let parsed = lease_seconds
            .parse::<u64>()
            .with_context(|| format!("parse x-codexw-lease-seconds `{lease_seconds}`"))?;
        object
            .entry("lease_seconds".to_string())
            .or_insert(Value::Number(parsed.into()));
    }

    Ok((
        Some("application/json".to_string()),
        serde_json::to_vec(&Value::Object(object)).context("serialize injected connector body")?,
    ))
}

pub(super) fn supports_client_lease_injection(local_path: &str) -> bool {
    let trimmed = local_path.trim_matches('/');
    let segments: Vec<&str> = if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    };
    matches!(
        segments.as_slice(),
        ["api", "v1", "session", "new"]
            | ["api", "v1", "session", "attach"]
            | ["api", "v1", "session", _, "attachment", "renew"]
            | ["api", "v1", "session", _, "attachment", "release"]
            | ["api", "v1", "session", _, "turn", "start"]
            | ["api", "v1", "session", _, "turn", "interrupt"]
            | ["api", "v1", "session", _, "shells", "start"]
            | ["api", "v1", "session", _, "shells", _, "poll"]
            | ["api", "v1", "session", _, "shells", _, "send"]
            | ["api", "v1", "session", _, "shells", _, "terminate"]
            | ["api", "v1", "session", _, "services", "update"]
            | ["api", "v1", "session", _, "services", _, "provide"]
            | ["api", "v1", "session", _, "services", _, "depend"]
            | ["api", "v1", "session", _, "services", _, "contract"]
            | ["api", "v1", "session", _, "services", _, "relabel"]
            | ["api", "v1", "session", _, "services", _, "attach"]
            | ["api", "v1", "session", _, "services", _, "wait"]
            | ["api", "v1", "session", _, "services", _, "run"]
    )
}

pub(super) fn compose_local_path(base: &Url, local_path: &str) -> String {
    let mut prefix = base.path().trim_end_matches('/').to_string();
    if prefix == "/" {
        prefix.clear();
    }
    format!("{prefix}{local_path}")
}

pub(super) fn write_upstream_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    content_type: Option<&str>,
    auth_token: Option<&str>,
    body: &[u8],
    last_event_id: Option<&str>,
) -> Result<()> {
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(content_type) = content_type {
        request.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    if let Some(auth_token) = auth_token {
        request.push_str(&format!("Authorization: Bearer {auth_token}\r\n"));
    }
    if let Some(last_event_id) = last_event_id {
        request.push_str(&format!("Last-Event-ID: {last_event_id}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .context("write upstream request head")?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .context("write upstream request body")?;
    }
    Ok(())
}

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
