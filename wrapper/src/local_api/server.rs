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
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;
use thiserror::Error;

use crate::Cli;

use super::LocalApiCommand;
use super::LocalApiEvent;
use super::LocalApiSnapshot;
use super::SharedCommandQueue;
use super::SharedEventLog;
use super::SharedSnapshot;
use super::control::enqueue_command;
use super::events::events_since;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const READ_TIMEOUT: Duration = Duration::from_millis(250);
const EVENT_STREAM_POLL_INTERVAL: Duration = Duration::from_millis(100);
const EVENT_STREAM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
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
    event_log: SharedEventLog,
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
                    let snapshot = snapshot.clone();
                    let command_queue = command_queue.clone();
                    let event_log = event_log.clone();
                    let auth_token = auth_token.clone();
                    thread::spawn(move || {
                        let _ = handle_connection(
                            stream,
                            &snapshot,
                            &command_queue,
                            &event_log,
                            auth_token.as_deref(),
                        );
                    });
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
    event_log: &SharedEventLog,
    auth_token: Option<&str>,
) -> Result<()> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set local API read timeout")?;
    let maybe_response = match read_request(&mut stream) {
        Ok(request) => {
            if let Some(response) = authorize_request(&request, auth_token) {
                Some(response)
            } else if is_event_stream_request(&request) {
                handle_event_stream_request(&mut stream, &request, snapshot, event_log)?;
                None
            } else {
                Some(route_authorized_request(&request, snapshot, command_queue))
            }
        }
        Err(RequestReadError::BadRequest) => Some(json_error_response(
            400,
            "bad_request",
            "invalid HTTP request",
        )),
        Err(RequestReadError::Io(_)) => return Ok(()),
    };
    if let Some(response) = maybe_response {
        write_response(&mut stream, &response)?;
    }
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
        if let Some(index) = request_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
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

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn route_request(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    auth_token: Option<&str>,
) -> HttpResponse {
    if let Some(response) = authorize_request(request, auth_token) {
        return response;
    }

    route_authorized_request(request, snapshot, command_queue)
}

fn authorize_request(request: &HttpRequest, auth_token: Option<&str>) -> Option<HttpResponse> {
    if request.path == "/healthz" && request.method == "GET" {
        return Some(json_ok_response(json!({ "ok": true })));
    }

    if let Some(expected_token) = auth_token {
        match request.headers.get("authorization") {
            Some(value) if value == &format!("Bearer {expected_token}") => {}
            _ => {
                return Some(json_error_response(
                    401,
                    "unauthorized",
                    "missing or invalid bearer token",
                ));
            }
        }
    }
    None
}

fn route_authorized_request(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    let current_snapshot = match snapshot.read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            return json_error_response(
                500,
                "snapshot_unavailable",
                "failed to access local API snapshot",
            );
        }
    };

    if request.method == "POST" && request.path == "/api/v1/turn/start" {
        return handle_turn_start_route(request, &current_snapshot, command_queue);
    }

    if request.method == "POST" && request.path == "/api/v1/turn/interrupt" {
        return handle_turn_interrupt_route(request, &current_snapshot, command_queue);
    }

    if request.method == "POST" {
        if let Some(path) = request.path.strip_prefix("/api/v1/session/") {
            return route_session_scoped_post(path, request, &current_snapshot, command_queue);
        }
        return json_error_response(404, "not_found", "unknown route");
    }

    if request.method != "GET" {
        return json_error_response(405, "method_not_allowed", "unsupported method for route");
    }

    if request.path == "/api/v1/session" {
        return json_ok_response(session_payload(&current_snapshot));
    }

    if let Some(path) = request.path.strip_prefix("/api/v1/session/") {
        return route_session_scoped_get(path, &current_snapshot);
    }

    json_error_response(404, "not_found", "unknown route")
}

fn route_session_scoped_get(path: &str, snapshot: &LocalApiSnapshot) -> HttpResponse {
    let mut parts = path.splitn(2, '/');
    let session_id = parts.next().unwrap_or_default();
    if session_id != snapshot.session_id {
        return json_error_response(404, "session_not_found", "unknown session id");
    }
    match parts.next() {
        None => json_ok_response(session_payload(snapshot)),
        Some("transcript") => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "transcript": snapshot.transcript,
        })),
        Some("orchestration/status") => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "orchestration": snapshot.orchestration_status,
        })),
        Some("orchestration/dependencies") => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "dependencies": snapshot.orchestration_dependencies,
        })),
        Some("orchestration/workers") => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "workers": snapshot.workers,
        })),
        Some("shells") => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "shells": snapshot.workers.background_shells,
        })),
        Some("services") => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "services": snapshot
                .workers
                .background_shells
                .iter()
                .filter(|job| job.intent == "service")
                .cloned()
                .collect::<Vec<_>>(),
        })),
        Some("capabilities") => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "capabilities": snapshot.capabilities,
        })),
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

fn route_session_scoped_post(
    path: &str,
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    let mut parts = path.splitn(2, '/');
    let session_id = parts.next().unwrap_or_default();
    if session_id != snapshot.session_id {
        return json_error_response(404, "session_not_found", "unknown session id");
    }
    match parts.next() {
        Some("shells/start") => {
            handle_shell_start_route(request, snapshot, command_queue, session_id)
        }
        Some("services/update") => {
            handle_service_update_route(request, snapshot, command_queue, session_id)
        }
        Some("dependencies/update") => {
            handle_dependency_update_route(request, snapshot, command_queue, session_id)
        }
        Some(rest) if rest.starts_with("shells/") => {
            route_shell_action_route(rest, request, snapshot, command_queue, session_id)
        }
        Some(rest) if rest.starts_with("services/") => {
            route_service_action_route(rest, request, snapshot, command_queue, session_id)
        }
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

fn route_shell_action_route(
    path: &str,
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    let Some(rest) = path.strip_prefix("shells/") else {
        return json_error_response(404, "not_found", "unknown route");
    };
    let mut parts = rest.splitn(2, '/');
    let Some(reference) = parts.next() else {
        return json_error_response(404, "not_found", "unknown route");
    };
    let Some(action) = parts.next() else {
        return json_error_response(404, "not_found", "unknown route");
    };
    match action {
        "poll" => handle_shell_poll_route(request, snapshot, reference, session_id),
        "send" => handle_shell_send_route(request, snapshot, command_queue, reference, session_id),
        "terminate" => {
            handle_shell_terminate_route(request, snapshot, command_queue, reference, session_id)
        }
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

fn route_service_action_route(
    path: &str,
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    let Some(rest) = path.strip_prefix("services/") else {
        return json_error_response(404, "not_found", "unknown route");
    };
    let mut parts = rest.splitn(2, '/');
    let Some(reference) = parts.next() else {
        return json_error_response(404, "not_found", "unknown route");
    };
    let Some(action) = parts.next() else {
        return json_error_response(404, "not_found", "unknown route");
    };
    match action {
        "provide" => {
            handle_service_provide_route(request, snapshot, command_queue, reference, session_id)
        }
        "depend" => {
            handle_service_depend_route(request, snapshot, command_queue, reference, session_id)
        }
        "contract" => {
            handle_service_contract_route(request, snapshot, command_queue, reference, session_id)
        }
        "relabel" => {
            handle_service_relabel_route(request, snapshot, command_queue, reference, session_id)
        }
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

fn is_event_stream_request(request: &HttpRequest) -> bool {
    request.path.ends_with("/events")
}

fn handle_event_stream_request(
    stream: &mut TcpStream,
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    event_log: &SharedEventLog,
) -> Result<()> {
    if request.method != "GET" {
        write_response(
            stream,
            &json_error_response(405, "method_not_allowed", "unsupported method for route"),
        )?;
        return Ok(());
    }

    let Some(path) = request.path.strip_prefix("/api/v1/session/") else {
        write_response(
            stream,
            &json_error_response(404, "not_found", "unknown route"),
        )?;
        return Ok(());
    };
    let Some(session_id) = path.strip_suffix("/events") else {
        write_response(
            stream,
            &json_error_response(404, "not_found", "unknown route"),
        )?;
        return Ok(());
    };

    let current_snapshot = match snapshot.read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            write_response(
                stream,
                &json_error_response(
                    500,
                    "snapshot_unavailable",
                    "failed to access local API snapshot",
                ),
            )?;
            return Ok(());
        }
    };

    if session_id != current_snapshot.session_id {
        write_response(
            stream,
            &json_error_response(404, "session_not_found", "unknown session id"),
        )?;
        return Ok(());
    }

    let last_event_id = request
        .headers
        .get("last-event-id")
        .and_then(|value| value.parse::<u64>().ok());
    write_event_stream_headers(stream)?;
    write_event_stream_comment(stream, "connected")?;

    let mut sent_event_id = last_event_id;
    let mut last_heartbeat = Instant::now();
    loop {
        let events = events_since(event_log, session_id, sent_event_id);
        for event in events {
            sent_event_id = Some(event.id);
            write_event_stream_event(stream, &event)?;
            last_heartbeat = Instant::now();
        }

        if last_heartbeat.elapsed() >= EVENT_STREAM_HEARTBEAT_INTERVAL {
            write_event_stream_comment(stream, "heartbeat")?;
            last_heartbeat = Instant::now();
        }

        thread::sleep(EVENT_STREAM_POLL_INTERVAL);
    }
}

fn write_event_stream_headers(stream: &mut TcpStream) -> Result<()> {
    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nX-Accel-Buffering: no\r\n\r\n",
        )
        .context("write local API event stream headers")?;
    stream
        .flush()
        .context("flush local API event stream headers")?;
    Ok(())
}

fn write_event_stream_comment(stream: &mut TcpStream, comment: &str) -> Result<()> {
    stream
        .write_all(format!(": {comment}\n\n").as_bytes())
        .context("write local API event stream comment")?;
    stream
        .flush()
        .context("flush local API event stream comment")?;
    Ok(())
}

fn write_event_stream_event(stream: &mut TcpStream, event: &LocalApiEvent) -> Result<()> {
    let data = serde_json::to_string(&event.data).context("serialize local API SSE event data")?;
    let payload = format!(
        "id: {}\nevent: {}\ndata: {}\n\n",
        event.id, event.event, data
    );
    stream
        .write_all(payload.as_bytes())
        .context("write local API event stream event")?;
    stream
        .flush()
        .context("flush local API event stream event")?;
    Ok(())
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

fn handle_shell_start_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    let Some(command) = object.get("command").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing command");
    };
    if command.trim().is_empty() {
        return json_error_response(400, "validation_error", "command must not be empty");
    }
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::StartShell {
            session_id: session_id.to_string(),
            arguments: body,
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue shell start: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
}

fn handle_shell_poll_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    if request.method != "POST" {
        return json_error_response(405, "method_not_allowed", "unsupported method for route");
    }
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    json_ok_response(json!({
        "ok": true,
        "session_id": session_id,
        "shell": shell,
    }))
}

fn handle_shell_send_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    let Some(text) = object.get("text").and_then(Value::as_str) else {
        return json_error_response(400, "validation_error", "missing text");
    };
    let append_newline = object
        .get("appendNewline")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::SendShellInput {
            session_id: session_id.to_string(),
            arguments: json!({
                "jobId": shell.id,
                "text": text,
                "appendNewline": append_newline,
            }),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue shell send: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "shell_id": shell.id,
    }))
}

fn handle_shell_terminate_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    if !request.body.is_empty() {
        if let Err(response) = json_request_body(request) {
            return response;
        }
    }
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::TerminateShell {
            session_id: session_id.to_string(),
            arguments: json!({ "jobId": shell.id }),
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue shell termination: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "shell_id": shell.id,
    }))
}

fn json_request_body(request: &HttpRequest) -> std::result::Result<Value, HttpResponse> {
    if request.body.is_empty() {
        return Err(json_error_response(
            400,
            "validation_error",
            "request body is required",
        ));
    }
    serde_json::from_slice::<Value>(&request.body).map_err(|_| {
        json_error_response(400, "validation_error", "request body must be valid JSON")
    })
}

fn resolve_shell_snapshot(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> std::result::Result<super::snapshot::LocalApiBackgroundShellJob, (&'static str, &'static str)>
{
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(("validation_error", "shell reference must not be empty"));
    }
    if let Some(shell) = snapshot
        .workers
        .background_shells
        .iter()
        .find(|shell| shell.id == reference)
    {
        return Ok(shell.clone());
    }
    if let Some(shell) = snapshot
        .workers
        .background_shells
        .iter()
        .find(|shell| shell.alias.as_deref() == Some(reference))
    {
        return Ok(shell.clone());
    }
    if let Some(capability) = reference.strip_prefix('@') {
        let capability_name = format!("@{capability}");
        let Some(entry) = snapshot
            .capabilities
            .iter()
            .find(|entry| entry.capability == capability_name)
        else {
            return Err(("shell_not_found", "unknown shell capability"));
        };
        if entry.providers.len() != 1 {
            return Err(("shell_reference_ambiguous", "shell capability is ambiguous"));
        }
        let job_id = &entry.providers[0].job_id;
        let Some(shell) = snapshot
            .workers
            .background_shells
            .iter()
            .find(|shell| &shell.id == job_id)
        else {
            return Err((
                "shell_not_found",
                "shell capability provider is not available",
            ));
        };
        return Ok(shell.clone());
    }
    if let Ok(index) = reference.parse::<usize>() {
        if index == 0 {
            return Err(("validation_error", "shell index must be 1-based"));
        }
        if let Some(shell) = snapshot.workers.background_shells.get(index - 1) {
            return Ok(shell.clone());
        }
    }
    Err(("shell_not_found", "unknown shell reference"))
}

fn handle_service_update_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("jobId") {
        return json_error_response(400, "validation_error", "missing jobId");
    }
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

fn handle_dependency_update_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    session_id: &str,
) -> HttpResponse {
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("jobId") {
        return json_error_response(400, "validation_error", "missing jobId");
    }
    if !object.contains_key("dependsOnCapabilities") {
        return json_error_response(400, "validation_error", "missing dependsOnCapabilities");
    }
    enqueue_dependency_update(command_queue, session_id, body, snapshot)
}

fn handle_service_provide_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("capabilities") {
        return json_error_response(400, "validation_error", "missing capabilities");
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

fn handle_service_depend_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("dependsOnCapabilities") {
        return json_error_response(400, "validation_error", "missing dependsOnCapabilities");
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_dependency_update(command_queue, session_id, body, snapshot)
}

fn handle_service_contract_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    let has_contract_field = object.contains_key("protocol")
        || object.contains_key("endpoint")
        || object.contains_key("attachHint")
        || object.contains_key("readyPattern")
        || object.contains_key("recipes");
    if !has_contract_field {
        return json_error_response(
            400,
            "validation_error",
            "contract update requires one of protocol, endpoint, attachHint, readyPattern, or recipes",
        );
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

fn handle_service_relabel_route(
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    reference: &str,
    session_id: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    let mut body = match json_request_body(request) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(object) = body.as_object_mut() else {
        return json_error_response(
            400,
            "validation_error",
            "request body must be a JSON object",
        );
    };
    if !object.contains_key("label") {
        return json_error_response(400, "validation_error", "missing label");
    }
    object.insert("jobId".to_string(), Value::String(shell.id.clone()));
    enqueue_service_update(command_queue, session_id, body, snapshot)
}

fn enqueue_service_update(
    command_queue: &SharedCommandQueue,
    session_id: &str,
    arguments: Value,
    snapshot: &LocalApiSnapshot,
) -> HttpResponse {
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::UpdateService {
            session_id: session_id.to_string(),
            arguments,
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue service update: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
}

fn enqueue_dependency_update(
    command_queue: &SharedCommandQueue,
    session_id: &str,
    arguments: Value,
    snapshot: &LocalApiSnapshot,
) -> HttpResponse {
    if let Err(err) = enqueue_command(
        command_queue,
        LocalApiCommand::UpdateDependencies {
            session_id: session_id.to_string(),
            arguments,
        },
    ) {
        return json_error_response(
            500,
            "queue_unavailable",
            &format!("failed to queue dependency update: {err:#}"),
        );
    }
    json_ok_response(json!({
        "ok": true,
        "accepted": true,
        "queued": true,
        "session_id": snapshot.session_id,
        "thread_id": snapshot.thread_id,
    }))
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
        "orchestration": snapshot.orchestration_status,
        "transcript_length": snapshot.transcript.len(),
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
