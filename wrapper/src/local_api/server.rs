use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use serde_json::json;
use serde_json::Value;
use thiserror::Error;

use crate::Cli;

use super::LocalApiCommand;
use super::LocalApiSnapshot;
use super::SharedCommandQueue;
use super::SharedSnapshot;
use super::control::enqueue_command;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const READ_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_REQUEST_BYTES: usize = 65536;

pub(crate) struct LocalApiHandle {
    bind_addr: String,
    stop: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl LocalApiHandle {
    pub(crate) fn bind_addr(&self) -> &str {
        &self.bind_addr
    }

    pub(crate) fn shutdown(mut self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            join_handle
                .join()
                .map_err(|_| anyhow::anyhow!("local API thread panicked"))?;
        }
        Ok(())
    }
}

pub(crate) fn start_local_api(
    cli: &Cli,
    snapshot: SharedSnapshot,
    command_queue: SharedCommandQueue,
) -> Result<Option<LocalApiHandle>> {
    if !cli.local_api {
        return Ok(None);
    }

    let listener = TcpListener::bind(&cli.local_api_bind)
        .with_context(|| format!("bind local API listener on `{}`", cli.local_api_bind))?;
    listener
        .set_nonblocking(true)
        .context("set local API listener nonblocking")?;
    let bind_addr = listener
        .local_addr()
        .context("read local API listener address")?
        .to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = Arc::clone(&stop);
    let auth_token = cli.local_api_token.clone();

    let join_handle = thread::spawn(move || {
        while !stop_for_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = handle_connection(stream, &snapshot, &command_queue, auth_token.as_deref());
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(ACCEPT_POLL_INTERVAL);
                }
                Err(_) => break,
            }
        }
    });

    Ok(Some(LocalApiHandle {
        bind_addr,
        stop,
        join_handle: Some(join_handle),
    }))
}

#[derive(Debug, Clone)]
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) reason: &'static str,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug, Error)]
enum RequestReadError {
    #[error("bad request")]
    BadRequest,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

fn handle_connection(
    mut stream: TcpStream,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    auth_token: Option<&str>,
) -> Result<()> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set local API read timeout")?;
    let response = match read_request(&mut stream) {
        Ok(request) => route_request(&request, snapshot, command_queue, auth_token),
        Err(RequestReadError::BadRequest) => json_error_response(400, "bad_request", "invalid HTTP request"),
        Err(RequestReadError::Io(_)) => return Ok(()),
    };
    write_response(&mut stream, &response)?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> std::result::Result<HttpRequest, RequestReadError> {
    let mut buffer = [0_u8; 1024];
    let mut request_bytes = Vec::new();
    let header_end = loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err(RequestReadError::BadRequest);
        }
        request_bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = request_bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        if request_bytes.len() >= MAX_REQUEST_BYTES {
            return Err(RequestReadError::BadRequest);
        }
    };
    let request_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let mut lines = request_text.split("\r\n");
    let request_line = lines.next().ok_or(RequestReadError::BadRequest)?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().ok_or(RequestReadError::BadRequest)?;
    let raw_path = request_parts.next().ok_or(RequestReadError::BadRequest)?;
    if request_parts.next().is_none() {
        return Err(RequestReadError::BadRequest);
    }

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(RequestReadError::BadRequest);
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    let content_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| RequestReadError::BadRequest)?
        .unwrap_or(0);
    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
        if header_end + body.len() >= MAX_REQUEST_BYTES {
            return Err(RequestReadError::BadRequest);
        }
    }
    if body.len() < content_length {
        return Err(RequestReadError::BadRequest);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method: method.to_string(),
        path: raw_path.split('?').next().unwrap_or(raw_path).to_string(),
        headers,
        body,
    })
}

pub(crate) fn route_request(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    auth_token: Option<&str>,
) -> HttpResponse {
    if request.path == "/healthz" && request.method == "GET" {
        return json_ok_response(json!({ "ok": true }));
    }

    if let Some(expected_token) = auth_token {
        match request.headers.get("authorization") {
            Some(value) if value == &format!("Bearer {expected_token}") => {}
            _ => {
                return json_error_response(401, "unauthorized", "missing or invalid bearer token");
            }
        }
    }

    let current_snapshot = match snapshot.read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            return json_error_response(500, "snapshot_unavailable", "failed to access local API snapshot");
        }
    };

    if request.method == "POST" && request.path == "/api/v1/turn/start" {
        return handle_turn_start_route(request, &current_snapshot, command_queue);
    }

    if request.method == "POST" && request.path == "/api/v1/turn/interrupt" {
        return handle_turn_interrupt_route(request, &current_snapshot, command_queue);
    }

    if request.method != "GET" {
        return json_error_response(405, "method_not_allowed", "unsupported method for route");
    }

    if request.path == "/api/v1/session" {
        return json_ok_response(session_payload(&current_snapshot));
    }

    if let Some(session_id) = request.path.strip_prefix("/api/v1/session/") {
        if session_id == current_snapshot.session_id {
            return json_ok_response(session_payload(&current_snapshot));
        }
        return json_error_response(404, "session_not_found", "unknown session id");
    }

    json_error_response(404, "not_found", "unknown route")
}

fn handle_turn_start_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(session_id) = body.get("session_id").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing session_id");
    };
    if session_id != snapshot.session_id {
        return json_error_response(404, "session_not_found", "unknown session id");
    }
    if snapshot.thread_id.is_none() {
        return json_error_response(409, "thread_not_attached", "session has no attached thread");
    }
    let Some(prompt) = body
        .get("input")
        .and_then(Value::as_object)
        .and_then(|input| input.get("text"))
        .and_then(Value::as_str)
    else {
        return json_error_response(400, "validation_error", "missing input.text");
    };
    if prompt.trim().is_empty() {
        return json_error_response(400, "validation_error", "input.text must not be empty");
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::StartTurn {
            session_id: session_id.to_string(),
            prompt: prompt.to_string(),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue start request: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
    }))
}

fn handle_turn_interrupt_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(session_id) = body.get("session_id").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing session_id");
    };
    if session_id != snapshot.session_id {
        return json_error_response(404, "session_not_found", "unknown session id");
    }
    if snapshot.active_turn_id.is_none() {
        return json_error_response(409, "turn_not_active", "session has no active turn");
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::InterruptTurn {
            session_id: session_id.to_string(),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue interrupt request: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
    }))
}

fn json_request_body(request: &HttpRequest) -> std::result::Result<Value, HttpResponse> {
    if request.body.is_empty() {
        return Err(json_error_response(400, "validation_error", "request body is required"));
    }
    serde_json::from_slice::<Value>(&request.body)
        .map_err(|_| json_error_response(400, "validation_error", "request body must be valid JSON"))
}

fn session_payload(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "cwd": snapshot.cwd,
        "thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
        "objective": snapshot.objective,
        "working": snapshot.turn_running,
        "started_turn_count": snapshot.started_turn_count,
        "completed_turn_count": snapshot.completed_turn_count,
        "active_personality": snapshot.active_personality,
    })
}

fn json_ok_response(body: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status: 200,
        reason: "OK",
        body: serde_json::to_vec_pretty(&body).unwrap_or_else(|_| b"{\"ok\":false}".to_vec()),
    }
}

fn json_error_response(status: u16, code: &str, message: &str) -> HttpResponse {
    let reason = match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Error",
    };
    json_ok_response(json!({
        "ok": false,
        "error": {
            "code": code,
            "message": message,
        }
    }))
    .with_status(status, reason)
}

impl HttpResponse {
    fn with_status(mut self, status: u16, reason: &'static str) -> Self {
        self.status = status;
        self.reason = reason;
        self
    }
}

fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<()> {
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        response.reason,
        response.body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .context("write local API response headers")?;
    stream
        .write_all(&response.body)
        .context("write local API response body")?;
    Ok(())
}
