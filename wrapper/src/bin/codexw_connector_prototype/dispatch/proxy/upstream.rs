use std::net::TcpStream;

use anyhow::Result;
use serde_json::json;

use super::super::super::Cli;
use super::super::super::http;
use super::super::super::upstream;
use super::super::gate::ProxyRequest;

pub(super) fn handle_proxy(
    stream: &mut TcpStream,
    cli: &Cli,
    proxy_request: &ProxyRequest,
) -> Result<()> {
    match upstream::forward_request(&proxy_request.request, cli, &proxy_request.target) {
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
