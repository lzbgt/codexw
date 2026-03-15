use std::net::TcpStream;

use anyhow::Context;

use super::super::MAX_REQUEST_BYTES;
use super::super::http_request_reader::DEFAULT_REQUEST_READ_DEADLINE;
use super::super::http_request_reader::ReadHttpRequestError;
use super::super::http_request_reader::read_http_request;
use super::HttpRequest;

pub(super) fn read_request(stream: &mut TcpStream) -> anyhow::Result<HttpRequest> {
    let request = match read_http_request(stream, MAX_REQUEST_BYTES, DEFAULT_REQUEST_READ_DEADLINE)
    {
        Ok(request) => request,
        Err(ReadHttpRequestError::BadRequest) => anyhow::bail!("invalid HTTP request"),
        Err(ReadHttpRequestError::Io(err)) => return Err(err).context("read connector request"),
    };
    Ok(HttpRequest {
        method: request.method,
        path: request.path,
        headers: request.headers,
        body: request.body,
    })
}
