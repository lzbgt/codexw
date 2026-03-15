#[path = "gate/auth.rs"]
mod auth;
#[path = "gate/health.rs"]
mod health;
#[path = "gate/route.rs"]
mod route;

use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;

use super::super::Cli;
use super::super::READ_TIMEOUT;
use super::super::http;

pub(crate) use route::ProxyRequest;

pub(super) enum ConnectionAction {
    Respond(http::HttpResponse),
    Proxy(route::ProxyRequest),
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

    if let Some(response) = health::health_response(&request, cli) {
        return Ok(ConnectionAction::Respond(response));
    }

    if let Some(response) = auth::auth_error(&request, cli) {
        return Ok(ConnectionAction::Respond(response));
    }

    match route::resolve_proxy_request(request, cli) {
        Ok(proxy_request) => Ok(ConnectionAction::Proxy(proxy_request)),
        Err(response) => Ok(ConnectionAction::Respond(response)),
    }
}
