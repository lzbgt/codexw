use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;
use serde_json::json;

use super::super::Cli;
use super::super::http;
use super::super::routing;
use super::super::sse;
use super::super::upstream;

pub(super) fn handle_proxy(
    stream: &mut TcpStream,
    cli: &Cli,
    request: &http::HttpRequest,
    target: &routing::ProxyTarget,
) -> Result<()> {
    if target.is_sse {
        return sse::handle_sse_proxy(
            stream.try_clone().context("clone client stream for SSE")?,
            request,
            cli,
            target,
        );
    }

    match upstream::forward_request(request, cli, target) {
        Ok(upstream) => {
            http::write_response(stream, &http::from_upstream_response(upstream, cli))?;
        }
        Err(upstream::ForwardRequestError::Validation { message, details }) => {
            http::write_response(
                stream,
                &http::json_error_response(400, "validation_error", &message, details),
            )?;
        }
        Err(upstream::ForwardRequestError::Transport(err)) => {
            http::write_response(
                stream,
                &http::json_error_response(
                    502,
                    "upstream_unavailable",
                    "connector could not reach or prepare the local API request",
                    Some(json!({
                        "cause": format!("{err:#}"),
                    })),
                ),
            )?;
        }
    }

    Ok(())
}
