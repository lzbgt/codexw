use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;
use serde_json::json;

use super::Cli;
use super::READ_TIMEOUT;
use super::http;
use super::routing;
use super::sse;
use super::upstream;

pub(super) fn handle_connection(stream: &mut TcpStream, cli: &Cli) -> Result<()> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set connector read timeout")?;
    let request = match http::read_request(stream) {
        Ok(request) => request,
        Err(_) => {
            http::write_response(
                stream,
                &http::json_error_response(400, "bad_request", "invalid HTTP request", None),
            )?;
            return Ok(());
        }
    };

    if request.method == "GET" && request.path == "/healthz" {
        http::write_response(
            stream,
            &http::json_ok_response(json!({
                "ok": true,
                "agent_id": cli.agent_id,
                "deployment_id": cli.deployment_id,
            })),
        )?;
        return Ok(());
    }

    if let Some(expected_token) = &cli.connector_token {
        match request.headers.get("authorization") {
            Some(value) if value == &format!("Bearer {expected_token}") => {}
            _ => {
                http::write_response(
                    stream,
                    &http::json_error_response(
                        401,
                        "unauthorized",
                        "missing or invalid connector bearer token",
                        None,
                    ),
                )?;
                return Ok(());
            }
        }
    }

    let Some(target) = routing::resolve_proxy_target(&request.method, &request.path, &cli.agent_id)
    else {
        http::write_response(
            stream,
            &http::json_error_response(404, "not_found", "unknown connector route", None),
        )?;
        return Ok(());
    };

    if target.is_sse && request.method != "GET" {
        http::write_response(
            stream,
            &http::json_error_response(
                405,
                "method_not_allowed",
                "unsupported method for SSE route",
                None,
            ),
        )?;
        return Ok(());
    }

    if !routing::is_allowed_local_proxy_target(&request.method, &target.local_path, target.is_sse) {
        http::write_response(
            stream,
            &http::json_error_response(
                403,
                "route_not_allowed",
                "connector route is outside the allowed local API surface",
                Some(json!({
                    "method": request.method,
                    "local_path": target.local_path,
                    "is_sse": target.is_sse,
                })),
            ),
        )?;
        return Ok(());
    }

    if target.is_sse {
        return sse::handle_sse_proxy(
            stream.try_clone().context("clone client stream for SSE")?,
            &request,
            cli,
            &target,
        );
    }

    match upstream::forward_request(&request, cli, &target) {
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
