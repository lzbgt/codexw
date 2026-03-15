use serde_json::Value;
use std::collections::HashMap;

#[path = "upstream/request.rs"]
mod request;
#[path = "upstream/response.rs"]
mod response;

#[derive(Debug, Clone)]
pub(super) struct UpstreamResponse {
    pub(super) status: u16,
    pub(super) reason: String,
    pub(super) headers: HashMap<String, String>,
    pub(super) body: Vec<u8>,
}

#[derive(Debug)]
pub(super) enum ForwardRequestError {
    Validation {
        message: String,
        details: Option<Value>,
    },
    Transport(anyhow::Error),
}

impl ForwardRequestError {
    pub(super) fn validation(message: impl Into<String>, details: Option<Value>) -> Self {
        Self::Validation {
            message: message.into(),
            details,
        }
    }
}

pub(super) fn forward_request(
    request: &super::http::HttpRequest,
    cli: &super::Cli,
    target: &super::routing::ProxyTarget,
) -> std::result::Result<UpstreamResponse, ForwardRequestError> {
    request::forward_request(request, cli, target)
}

#[cfg(test)]
pub(super) fn prepare_upstream_body(
    request: &super::http::HttpRequest,
    target: &super::routing::ProxyTarget,
) -> std::result::Result<(Option<String>, Vec<u8>), ForwardRequestError> {
    request::prepare_upstream_body(request, target)
}

pub(super) fn compose_local_path(base: &url::Url, local_path: &str) -> String {
    request::compose_local_path(base, local_path)
}

pub(super) fn write_upstream_request(
    stream: &mut std::net::TcpStream,
    method: &str,
    path: &str,
    content_type: Option<&str>,
    auth_token: Option<&str>,
    body: &[u8],
    last_event_id: Option<&str>,
) -> anyhow::Result<()> {
    request::write_upstream_request(
        stream,
        method,
        path,
        content_type,
        auth_token,
        body,
        last_event_id,
    )
}

pub(super) fn read_upstream_response(
    stream: std::net::TcpStream,
) -> anyhow::Result<UpstreamResponse> {
    response::read_upstream_response(stream)
}

pub(super) fn read_error_body(
    status: u16,
    reason: String,
    headers: HashMap<String, String>,
    remainder: Vec<u8>,
    stream: std::net::TcpStream,
) -> anyhow::Result<UpstreamResponse> {
    response::read_error_body(status, reason, headers, remainder, stream)
}

pub(super) fn read_upstream_head(
    stream: &mut std::net::TcpStream,
) -> anyhow::Result<(u16, String, HashMap<String, String>, Vec<u8>)> {
    response::read_upstream_head(stream)
}
