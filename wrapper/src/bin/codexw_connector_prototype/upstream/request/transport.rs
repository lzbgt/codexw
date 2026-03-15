#[path = "transport/path.rs"]
mod path;
#[path = "transport/wire.rs"]
mod wire;

use std::net::TcpStream;

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

    let base =
        wire::parse_local_api_base(&cli.local_api_base).map_err(ForwardRequestError::Transport)?;
    let mut stream =
        wire::connect_upstream_stream(&base).map_err(ForwardRequestError::Transport)?;

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
    path::compose_local_path(base, local_path)
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
    wire::write_upstream_request(
        stream,
        method,
        path,
        content_type,
        auth_token,
        body,
        last_event_id,
    )
}
