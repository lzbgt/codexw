use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use serde_json::Value;
use serde_json::json;
use url::Url;

use super::super::Cli;
use super::super::http::HttpRequest;
use super::super::routing::ProxyTarget;
use super::super::routing::supports_client_lease_injection;
use super::ForwardRequestError;
use super::UpstreamResponse;
use super::read_upstream_response;

pub(super) fn forward_request(
    request: &HttpRequest,
    cli: &Cli,
    target: &ProxyTarget,
) -> std::result::Result<UpstreamResponse, ForwardRequestError> {
    let (content_type, body) = prepare_upstream_body(request, target)?;

    let base = Url::parse(&cli.local_api_base)
        .context("parse local API base URL")
        .map_err(ForwardRequestError::Transport)?;
    let host = base
        .host_str()
        .context("local API base URL missing host")
        .map_err(ForwardRequestError::Transport)?
        .to_string();
    let port = base
        .port_or_known_default()
        .context("local API base URL missing port")
        .map_err(ForwardRequestError::Transport)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("connect to local API {}:{}", host, port))
        .map_err(ForwardRequestError::Transport)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .context("set upstream read timeout")
        .map_err(ForwardRequestError::Transport)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("set upstream write timeout")
        .map_err(ForwardRequestError::Transport)?;

    let path = compose_local_path(&base, &target.local_path);
    write_upstream_request(
        &mut stream,
        &request.method,
        &path,
        content_type.as_deref(),
        cli.local_api_token.as_deref(),
        body.as_slice(),
        request.headers.get("last-event-id").map(String::as_str),
    )
    .map_err(ForwardRequestError::Transport)?;

    read_upstream_response(stream).map_err(ForwardRequestError::Transport)
}

pub(super) fn prepare_upstream_body(
    request: &HttpRequest,
    target: &ProxyTarget,
) -> std::result::Result<(Option<String>, Vec<u8>), ForwardRequestError> {
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

    let requires_object_body = target.session_id_hint.is_some()
        || supports_client_lease_injection(&request.method, &target.local_path);
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
        let value: Value = serde_json::from_slice(&request.body).map_err(|_| {
            ForwardRequestError::validation(
                "connector JSON injection requires a JSON object body",
                Some(json!({
                    "field": "body",
                    "expected": "json object",
                })),
            )
        })?;
        let Some(object) = value.as_object() else {
            return Err(ForwardRequestError::validation(
                "connector JSON injection requires a JSON object body",
                Some(json!({
                    "field": "body",
                    "expected": "json object",
                })),
            ));
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
        let parsed = lease_seconds.parse::<u64>().map_err(|_| {
            ForwardRequestError::validation(
                "x-codexw-lease-seconds must be a positive integer header",
                Some(json!({
                    "field": "x-codexw-lease-seconds",
                    "expected": "positive integer header",
                })),
            )
        })?;
        object
            .entry("lease_seconds".to_string())
            .or_insert(Value::Number(parsed.into()));
    }

    Ok((
        Some("application/json".to_string()),
        serde_json::to_vec(&Value::Object(object))
            .context("serialize injected connector body")
            .map_err(ForwardRequestError::Transport)?,
    ))
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
) -> anyhow::Result<()> {
    use std::io::Write;

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
