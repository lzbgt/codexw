use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;

use super::super::super::Cli;
use super::super::super::sse;
use super::super::gate::ProxyRequest;

pub(super) fn handle_proxy(
    stream: &mut TcpStream,
    cli: &Cli,
    proxy_request: &ProxyRequest,
) -> Result<()> {
    sse::handle_sse_proxy(
        stream.try_clone().context("clone client stream for SSE")?,
        &proxy_request.request,
        cli,
        &proxy_request.target,
    )
}
