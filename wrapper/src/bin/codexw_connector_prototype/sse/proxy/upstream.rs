use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use url::Url;

use super::super::super::Cli;
use super::super::super::http::HttpRequest;
use super::super::super::routing::ProxyTarget;
use super::super::super::upstream::UpstreamResponse;
use super::super::super::upstream::compose_local_path;
use super::super::super::upstream::read_error_body;
use super::super::super::upstream::read_upstream_head;
use super::super::super::upstream::write_upstream_request;

pub(super) fn open_upstream_sse_stream(
    request: &HttpRequest,
    cli: &Cli,
    target: &ProxyTarget,
) -> Result<(
    u16,
    String,
    std::collections::HashMap<String, String>,
    Vec<u8>,
    TcpStream,
)> {
    let base = Url::parse(&cli.local_api_base).context("parse local API base URL")?;
    let host = base
        .host_str()
        .context("local API base URL missing host")?
        .to_string();
    let port = base
        .port_or_known_default()
        .context("local API base URL missing port")?;
    let mut upstream_stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("connect to local API {}:{}", host, port))?;
    upstream_stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .context("set upstream SSE read timeout")?;
    upstream_stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("set upstream SSE write timeout")?;

    let path = compose_local_path(&base, &target.local_path);
    write_upstream_request(
        &mut upstream_stream,
        "GET",
        &path,
        None,
        cli.local_api_token.as_deref(),
        &[],
        request.headers.get("last-event-id").map(String::as_str),
    )?;

    let (status, reason, headers, remainder) = read_upstream_head(&mut upstream_stream)?;
    Ok((status, reason, headers, remainder, upstream_stream))
}

pub(super) fn read_non_success_response(
    status: u16,
    reason: String,
    headers: std::collections::HashMap<String, String>,
    remainder: Vec<u8>,
    stream: TcpStream,
) -> Result<UpstreamResponse> {
    read_error_body(status, reason, headers, remainder, stream)
}
