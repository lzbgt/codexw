use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;
use serde_json::json;

use super::super::Cli;
use super::super::READ_TIMEOUT;
use super::super::http;
use super::super::routing;

pub(super) enum ConnectionAction {
    Respond(http::HttpResponse),
    Proxy {
        request: http::HttpRequest,
        target: routing::ProxyTarget,
    },
}

pub(super) fn prepare_request(stream: &mut TcpStream, cli: &Cli) -> Result<ConnectionAction> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set connector read timeout")?;
    let request = match http::read_request(stream) {
        Ok(request) => request,
        Err(_) => {
            return Ok(ConnectionAction::Respond(http::json_error_response(
                400,
                "bad_request",
                "invalid HTTP request",
                None,
            )));
        }
    };

    if request.method == "GET" && request.path == "/healthz" {
        return Ok(ConnectionAction::Respond(http::json_ok_response(json!({
            "ok": true,
            "agent_id": cli.agent_id,
            "deployment_id": cli.deployment_id,
        }))));
    }

    if let Some(expected_token) = &cli.connector_token {
        match request.headers.get("authorization") {
            Some(value) if value == &format!("Bearer {expected_token}") => {}
            _ => {
                return Ok(ConnectionAction::Respond(http::json_error_response(
                    401,
                    "unauthorized",
                    "missing or invalid connector bearer token",
                    None,
                )));
            }
        }
    }

    let Some(target) = routing::resolve_proxy_target(&request.method, &request.path, &cli.agent_id)
    else {
        return Ok(ConnectionAction::Respond(http::json_error_response(
            404,
            "not_found",
            "unknown connector route",
            None,
        )));
    };

    if target.is_sse && request.method != "GET" {
        return Ok(ConnectionAction::Respond(http::json_error_response(
            405,
            "method_not_allowed",
            "unsupported method for SSE route",
            None,
        )));
    }

    if !routing::is_allowed_local_proxy_target(&request.method, &target.local_path, target.is_sse) {
        return Ok(ConnectionAction::Respond(http::json_error_response(
            403,
            "route_not_allowed",
            "connector route is outside the allowed local API surface",
            Some(json!({
                "method": request.method,
                "local_path": target.local_path,
                "is_sse": target.is_sse,
            })),
        )));
    }

    Ok(ConnectionAction::Proxy { request, target })
}
