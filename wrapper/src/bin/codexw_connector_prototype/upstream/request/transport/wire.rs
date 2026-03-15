use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use url::Url;

pub(super) fn parse_local_api_base(local_api_base: &str) -> Result<Url> {
    Url::parse(local_api_base).context("parse local API base URL")
}

pub(super) fn connect_upstream_stream(base: &Url) -> Result<TcpStream> {
    let host = base
        .host_str()
        .context("local API base URL missing host")?
        .to_string();
    let port = base
        .port_or_known_default()
        .context("local API base URL missing port")?;
    let stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("connect to local API {}:{}", host, port))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .context("set upstream read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("set upstream write timeout")?;
    Ok(stream)
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
