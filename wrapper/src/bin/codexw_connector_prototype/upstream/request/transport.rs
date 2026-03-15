use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use url::Url;

use super::super::super::Cli;
use super::super::super::http::HttpRequest;
use super::super::super::routing::ProxyTarget;
use super::super::ForwardRequestError;
use super::super::UpstreamResponse;
use super::super::read_upstream_response;
use super::body::prepare_upstream_body;

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
