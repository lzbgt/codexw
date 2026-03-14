use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;
use crate::http_request_reader::ReadHttpRequestError;
use crate::http_request_reader::read_http_request;

use super::Cli;
use super::MAX_REQUEST_BYTES;
use super::upstream::UpstreamResponse;

const REQUEST_READ_DEADLINE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: HashMap<String, String>,
    pub(super) body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(super) struct HttpResponse {
    pub(super) status: u16,
    pub(super) reason: &'static str,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

pub(super) fn from_upstream_response(upstream: UpstreamResponse, cli: &Cli) -> HttpResponse {
    let mut headers = Vec::new();
    if let Some(content_type) = upstream.headers.get("content-type") {
        headers.push(("Content-Type".to_string(), content_type.clone()));
    } else {
        headers.push((
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        ));
    }
    headers.push(("X-Codexw-Agent-Id".to_string(), cli.agent_id.clone()));
    headers.push((
        "X-Codexw-Deployment-Id".to_string(),
        cli.deployment_id.clone(),
    ));
    headers.push((
        HEADER_BROKER_ADAPTER_VERSION.to_string(),
        CODEXW_BROKER_ADAPTER_VERSION.to_string(),
    ));
    if let Some(local_api_version) = upstream.headers.get("x-codexw-local-api-version") {
        headers.push((
            HEADER_LOCAL_API_VERSION.to_string(),
            local_api_version.clone(),
        ));
    }
    HttpResponse {
        status: upstream.status,
        reason: Box::leak(upstream.reason.into_boxed_str()),
        headers,
        body: upstream.body,
    }
}

pub(super) fn read_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let request = match read_http_request(stream, MAX_REQUEST_BYTES, REQUEST_READ_DEADLINE) {
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

pub(super) fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.reason,
        response.body.len()
    );
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .context("write response head")?;
    if !response.body.is_empty() {
        stream
            .write_all(&response.body)
            .context("write response body")?;
    }
    Ok(())
}

pub(super) fn json_ok_response(body: Value) -> HttpResponse {
    let body = match body {
        Value::Object(mut object) => {
            object.insert(
                "broker_adapter_version".to_string(),
                Value::String(CODEXW_BROKER_ADAPTER_VERSION.to_string()),
            );
            Value::Object(object)
        }
        other => other,
    };
    HttpResponse {
        status: 200,
        reason: "OK",
        headers: vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string(),
            ),
        ],
        body: serde_json::to_vec(&body).expect("serialize ok response"),
    }
}

pub(super) fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
    details: Option<Value>,
) -> HttpResponse {
    let mut error = json!({
        "status": status,
        "code": code,
        "message": message,
    });
    if let Some(details) = details {
        error["details"] = details;
    }
    let body = json!({
        "ok": false,
        "broker_adapter_version": CODEXW_BROKER_ADAPTER_VERSION,
        "error": error,
    });
    HttpResponse {
        status,
        reason: match status {
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            405 => "Method Not Allowed",
            502 => "Bad Gateway",
            _ => "Error",
        },
        headers: vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string(),
            ),
        ],
        body: serde_json::to_vec(&body).expect("serialize error response"),
    }
}
