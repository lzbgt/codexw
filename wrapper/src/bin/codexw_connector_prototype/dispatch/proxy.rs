#[path = "proxy/sse.rs"]
mod sse;
#[path = "proxy/upstream.rs"]
mod upstream;

use std::net::TcpStream;

use anyhow::Result;

use super::super::Cli;
use super::gate::ProxyRequest;

pub(super) fn handle_proxy(
    stream: &mut TcpStream,
    cli: &Cli,
    proxy_request: &ProxyRequest,
) -> Result<()> {
    if proxy_request.target.is_sse {
        return sse::handle_proxy(stream, cli, proxy_request);
    }

    upstream::handle_proxy(stream, cli, proxy_request)
}
