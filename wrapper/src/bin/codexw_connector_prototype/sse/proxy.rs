#[path = "proxy/stream.rs"]
mod stream;
#[path = "proxy/upstream.rs"]
mod upstream;

use std::net::Shutdown;
use std::net::TcpStream;

use anyhow::Result;

use super::super::Cli;
use super::super::http::HttpRequest;
use super::super::http::from_upstream_response;
use super::super::http::json_error_response;
use super::super::http::write_response;
use super::super::routing::ProxyTarget;

pub(super) fn handle_sse_proxy(
    mut client_stream: TcpStream,
    request: &HttpRequest,
    cli: &Cli,
    target: &ProxyTarget,
) -> Result<()> {
    if request.method != "GET" {
        write_response(
            &mut client_stream,
            &json_error_response(
                405,
                "method_not_allowed",
                "unsupported method for SSE route",
                None,
            ),
        )?;
        let _ = client_stream.shutdown(Shutdown::Both);
        return Ok(());
    }

    let (status, reason, headers, remainder, upstream_stream) =
        upstream::open_upstream_sse_stream(request, cli, target)?;
    if status != 200 {
        let upstream = upstream::read_non_success_response(
            status,
            reason,
            headers,
            remainder,
            upstream_stream,
        )?;
        write_response(&mut client_stream, &from_upstream_response(upstream, cli))?;
        let _ = client_stream.shutdown(Shutdown::Both);
        return Ok(());
    }

    stream::bridge_upstream_events(&mut client_stream, upstream_stream, headers, remainder, cli)?;

    let _ = client_stream.shutdown(Shutdown::Both);
    Ok(())
}

#[cfg(test)]
pub(super) fn connector_sse_response_head(cli: &Cli, local_api_version: Option<&str>) -> String {
    stream::connector_sse_response_head(cli, local_api_version)
}
