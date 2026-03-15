#[path = "http/request.rs"]
mod request;
#[path = "http/response.rs"]
mod response;
#[path = "http/types.rs"]
mod types;

pub(crate) use types::HttpRequest;
pub(crate) use types::HttpResponse;

pub(super) fn from_upstream_response(
    upstream: super::upstream::UpstreamResponse,
    cli: &super::Cli,
) -> HttpResponse {
    response::from_upstream_response(upstream, cli)
}

pub(super) fn read_request(stream: &mut std::net::TcpStream) -> anyhow::Result<HttpRequest> {
    request::read_request(stream)
}

pub(super) fn write_response(
    stream: &mut std::net::TcpStream,
    response: &HttpResponse,
) -> anyhow::Result<()> {
    response::write_response(stream, response)
}

pub(super) fn json_ok_response(body: serde_json::Value) -> HttpResponse {
    response::json_ok_response(body)
}

pub(super) fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
    details: Option<serde_json::Value>,
) -> HttpResponse {
    response::json_error_response(status, code, message, details)
}
