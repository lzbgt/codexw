#[path = "request/body.rs"]
mod body;
#[path = "request/transport.rs"]
mod transport;

use std::net::TcpStream;

use anyhow::Result;
use url::Url;

use super::super::Cli;
use super::super::http::HttpRequest;
use super::super::routing::ProxyTarget;
use super::ForwardRequestError;
use super::UpstreamResponse;

pub(super) fn forward_request(
    request: &HttpRequest,
    cli: &Cli,
    target: &ProxyTarget,
) -> std::result::Result<UpstreamResponse, ForwardRequestError> {
    transport::forward_request(request, cli, target)
}

#[cfg(test)]
pub(super) fn prepare_upstream_body(
    request: &HttpRequest,
    target: &ProxyTarget,
) -> std::result::Result<(Option<String>, Vec<u8>), ForwardRequestError> {
    body::prepare_upstream_body(request, target)
}

pub(super) fn compose_local_path(base: &Url, local_path: &str) -> String {
    transport::compose_local_path(base, local_path)
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
    transport::write_upstream_request(
        stream,
        method,
        path,
        content_type,
        auth_token,
        body,
        last_event_id,
    )
}
