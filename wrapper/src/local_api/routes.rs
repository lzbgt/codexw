mod orchestration;
mod services;
mod session;
mod shells;
mod transcript;
mod turn;

use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::background_shells::BackgroundShellManager;

use super::LocalApiEvent;
use super::LocalApiSnapshot;
use super::SharedCommandQueue;
use super::SharedEventLog;
use super::SharedSnapshot;
use super::events::events_since;
use super::server::HttpRequest;
use super::server::HttpResponse;
use super::server::write_response;
use super::snapshot::local_api_shell_job;

const EVENT_STREAM_POLL_INTERVAL: Duration = Duration::from_millis(100);
const EVENT_STREAM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn route_request(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    auth_token: Option<&str>,
) -> HttpResponse {
    let background_shells = BackgroundShellManager::default();
    route_request_with_manager(
        request,
        snapshot,
        command_queue,
        &background_shells,
        auth_token,
    )
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn route_request_with_manager(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    background_shells: &BackgroundShellManager,
    auth_token: Option<&str>,
) -> HttpResponse {
    if let Some(response) = authorize_request(request, auth_token) {
        return response;
    }

    route_authorized_request(request, snapshot, command_queue, background_shells)
}

pub(super) fn authorize_request(
    request: &HttpRequest,
    auth_token: Option<&str>,
) -> Option<HttpResponse> {
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

pub(super) fn route_authorized_request(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    background_shells: &BackgroundShellManager,
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
        return turn::handle_turn_start_route(request, &current_snapshot, command_queue);
    }

    if request.method == "POST" && request.path == "/api/v1/turn/interrupt" {
        return turn::handle_turn_interrupt_route(request, &current_snapshot, command_queue);
    }

    if request.method == "POST" && request.path == "/api/v1/session/new" {
        return session::handle_session_new_route(request, &current_snapshot, command_queue);
    }

    if request.method == "POST" && request.path == "/api/v1/session/attach" {
        return session::handle_session_attach_route(request, &current_snapshot, command_queue);
    }

    if request.method == "POST" {
        if let Some(path) = request.path.strip_prefix("/api/v1/session/") {
            return route_session_scoped_post(
                path,
                request,
                &current_snapshot,
                command_queue,
                background_shells,
            );
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
    let rest = parts.next().unwrap_or_default();
    match rest {
        "" => json_ok_response(session_payload(snapshot)),
        "transcript" => transcript::handle_transcript_route(snapshot),
        "orchestration/status" => orchestration::handle_orchestration_status_route(snapshot),
        "orchestration/dependencies" => {
            orchestration::handle_orchestration_dependencies_route(snapshot)
        }
        "orchestration/workers" => orchestration::handle_orchestration_workers_route(snapshot),
        "shells" => shells::handle_shells_route(snapshot),
        "services" => services::handle_services_route(snapshot),
        "capabilities" => services::handle_capabilities_route(snapshot),
        _ if rest.starts_with("shells/") => {
            shells::handle_shell_detail_route(snapshot, &rest["shells/".len()..])
        }
        _ if rest.starts_with("services/") => {
            services::handle_service_detail_route(snapshot, &rest["services/".len()..])
        }
        _ if rest.starts_with("capabilities/") => {
            services::handle_capability_detail_route(snapshot, &rest["capabilities/".len()..])
        }
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

fn route_session_scoped_post(
    path: &str,
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    background_shells: &BackgroundShellManager,
) -> HttpResponse {
    let mut parts = path.splitn(2, '/');
    let session_id = parts.next().unwrap_or_default();
    if session_id != snapshot.session_id {
        return json_error_response(404, "session_not_found", "unknown session id");
    }
    let rest = parts.next().unwrap_or_default();
    match rest {
        "turn/start" => turn::handle_turn_start_route_for_session(request, snapshot, command_queue),
        "turn/interrupt" => {
            turn::handle_turn_interrupt_route_for_session(request, snapshot, command_queue)
        }
        "attachment/renew" => {
            session::handle_attachment_renew_route(request, snapshot, command_queue)
        }
        "attachment/release" => {
            session::handle_attachment_release_route(request, snapshot, command_queue)
        }
        "shells/start" => shells::handle_shell_start_route(request, snapshot, command_queue),
        "services/update" => {
            services::handle_service_update_route(request, snapshot, command_queue, session_id)
        }
        "dependencies/update" => {
            services::handle_dependency_update_route(request, snapshot, command_queue, session_id)
        }
        _ if rest.starts_with("shells/") => {
            route_shell_action_route(&rest["shells/".len()..], request, snapshot, command_queue)
        }
        _ if rest.starts_with("services/") => route_service_action_route(
            &rest["services/".len()..],
            request,
            snapshot,
            command_queue,
            background_shells,
            session_id,
        ),
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

fn route_shell_action_route(
    path: &str,
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
) -> HttpResponse {
    let Some((reference, action)) = path.rsplit_once('/') else {
        return json_error_response(404, "not_found", "unknown route");
    };
    match action {
        "poll" => shells::handle_shell_poll_route(snapshot, reference),
        "send" => shells::handle_shell_send_route(request, snapshot, command_queue, reference),
        "terminate" => {
            shells::handle_shell_terminate_route(request, snapshot, command_queue, reference)
        }
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

fn route_service_action_route(
    path: &str,
    request: &HttpRequest,
    snapshot: &LocalApiSnapshot,
    command_queue: &SharedCommandQueue,
    background_shells: &BackgroundShellManager,
    session_id: &str,
) -> HttpResponse {
    let Some((reference, action)) = path.rsplit_once('/') else {
        return json_error_response(404, "not_found", "unknown route");
    };
    match action {
        "attach" => services::handle_service_attach_route(
            request,
            snapshot,
            background_shells,
            reference,
            session_id,
        ),
        "wait" => services::handle_service_wait_route(
            request,
            snapshot,
            background_shells,
            reference,
            session_id,
        ),
        "run" => services::handle_service_run_route(
            request,
            snapshot,
            background_shells,
            reference,
            session_id,
        ),
        "provide" => services::handle_service_provide_route(
            request,
            snapshot,
            command_queue,
            reference,
            session_id,
        ),
        "depend" => services::handle_service_depend_route(
            request,
            snapshot,
            command_queue,
            reference,
            session_id,
        ),
        "contract" => services::handle_service_contract_route(
            request,
            snapshot,
            command_queue,
            reference,
            session_id,
        ),
        "relabel" => services::handle_service_relabel_route(
            request,
            snapshot,
            command_queue,
            reference,
            session_id,
        ),
        _ => json_error_response(404, "not_found", "unknown route"),
    }
}

pub(super) fn is_event_stream_request(request: &HttpRequest) -> bool {
    request.path.ends_with("/events")
}

pub(super) fn handle_event_stream_request(
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

pub(super) fn json_request_body(request: &HttpRequest) -> std::result::Result<Value, HttpResponse> {
    serde_json::from_slice(&request.body)
        .map_err(|_| json_error_response(400, "invalid_json", "request body must be valid JSON"))
}

pub(super) fn resolve_shell_snapshot(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> std::result::Result<
    crate::local_api::snapshot::LocalApiBackgroundShellJob,
    (&'static str, &'static str),
> {
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
        let matches: Vec<_> = snapshot
            .capabilities
            .iter()
            .filter(|entry| entry.capability.trim_start_matches('@') == capability)
            .flat_map(|entry| entry.providers.iter())
            .filter_map(|provider| {
                snapshot
                    .workers
                    .background_shells
                    .iter()
                    .find(|shell| shell.id == provider.job_id)
                    .cloned()
            })
            .collect();
        return match matches.as_slice() {
            [shell] => Ok(shell.clone()),
            [] => Err(("shell_not_found", "unknown shell reference")),
            _ => Err(("shell_reference_ambiguous", "shell reference is ambiguous")),
        };
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

pub(super) fn current_shell_value(
    background_shells: &BackgroundShellManager,
    shell_id: &str,
) -> Option<Value> {
    background_shells
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.id == shell_id)
        .map(local_api_shell_job)
        .and_then(|shell| serde_json::to_value(shell).ok())
}

pub(super) fn session_payload(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    let session = session_summary(snapshot);
    json!({
        "ok": true,
        "session": session,
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

pub(super) fn attachment_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    let lease_active = snapshot
        .attachment_lease_expires_at_ms
        .is_some_and(|expiry| expiry > now_unix_ms());
    json!({
        "id": format!("attach:{}", snapshot.session_id),
        "scope": "process",
        "process_scoped": true,
        "client_id": snapshot.attachment_client_id,
        "lease_seconds": snapshot.attachment_lease_seconds,
        "lease_expires_at_ms": snapshot.attachment_lease_expires_at_ms,
        "lease_active": lease_active,
        "attached_thread_id": snapshot.thread_id,
    })
}

pub(super) fn session_summary(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    json!({
        "id": snapshot.session_id,
        "scope": "process",
        "process_scoped": true,
        "attachment": attachment_summary(snapshot),
        "client_id": snapshot.attachment_client_id,
        "cwd": snapshot.cwd,
        "attached_thread_id": snapshot.thread_id,
        "active_turn_id": snapshot.active_turn_id,
        "objective": snapshot.objective,
        "working": snapshot.turn_running,
        "started_turn_count": snapshot.started_turn_count,
        "completed_turn_count": snapshot.completed_turn_count,
        "active_personality": snapshot.active_personality,
        "transcript_length": snapshot.transcript.len(),
    })
}

pub(super) fn attachment_has_active_conflicting_client(
    snapshot: &LocalApiSnapshot,
    requested_client_id: Option<&str>,
) -> bool {
    let Some(existing_client_id) = snapshot.attachment_client_id.as_deref() else {
        return false;
    };
    if !attachment_lease_active(snapshot) {
        return false;
    }
    match requested_client_id {
        Some(requested_client_id) => existing_client_id != requested_client_id,
        None => true,
    }
}

fn attachment_lease_active(snapshot: &LocalApiSnapshot) -> bool {
    snapshot
        .attachment_lease_expires_at_ms
        .is_some_and(|expiry| expiry > now_unix_ms())
}

pub(super) fn parse_optional_client_id(
    body: &Value,
) -> Result<Option<String>, crate::local_api::server::HttpResponse> {
    let Some(value) = body.get("client_id") else {
        return Ok(None);
    };
    let Some(client_id) = value.as_str() else {
        return Err(json_error_response_with_details(
            400,
            "validation_error",
            "client_id must be a string",
            json!({
                "field": "client_id",
                "expected": "string",
            }),
        ));
    };
    let trimmed = client_id.trim();
    if trimmed.is_empty() {
        return Err(json_error_response_with_details(
            400,
            "validation_error",
            "client_id must not be empty",
            json!({
                "field": "client_id",
                "expected": "non-empty string",
            }),
        ));
    }
    Ok(Some(trimmed.to_string()))
}

pub(super) fn enforce_attachment_lease_ownership(
    snapshot: &LocalApiSnapshot,
    requested_client_id: Option<&str>,
) -> Result<(), crate::local_api::server::HttpResponse> {
    if attachment_has_active_conflicting_client(snapshot, requested_client_id) {
        return Err(json_error_response_with_details(
            409,
            "attachment_conflict",
            "another client currently holds the active attachment lease",
            json!({
                "session_id": snapshot.session_id,
                "requested_client_id": requested_client_id,
                "current_attachment": {
                    "client_id": snapshot.attachment_client_id,
                    "lease_seconds": snapshot.attachment_lease_seconds,
                    "lease_expires_at_ms": snapshot.attachment_lease_expires_at_ms,
                    "lease_active": attachment_lease_active(snapshot),
                }
            }),
        ));
    }
    Ok(())
}

pub(super) fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .ok()
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(0)
}

pub(super) fn json_ok_response(body: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status: 200,
        reason: "OK",
        body: serde_json::to_vec_pretty(&body).unwrap_or_else(|_| b"{\"ok\":false}".to_vec()),
    }
}

pub(super) fn json_error_response(status: u16, code: &str, message: &str) -> HttpResponse {
    json_error_response_with_details(status, code, message, json!({}))
}

pub(super) fn json_error_response_with_details(
    status: u16,
    code: &str,
    message: &str,
    details: serde_json::Value,
) -> HttpResponse {
    let reason = match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "Error",
    };
    json_ok_response(json!({
        "ok": false,
        "error": {
            "status": status,
            "code": code,
            "message": message,
            "retryable": status >= 500,
            "details": details,
        }
    }))
    .with_status(status, reason)
}

impl HttpResponse {
    pub(super) fn with_status(mut self, status: u16, reason: &'static str) -> Self {
        self.status = status;
        self.reason = reason;
        self
    }
}
