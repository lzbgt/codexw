use crate::background_shells::BackgroundShellManager;
use crate::local_api::LocalApiSnapshot;
use crate::local_api::SharedCommandQueue;
use crate::local_api::SharedEventLog;
use crate::local_api::SharedSnapshot;
use crate::local_api::new_event_log;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;

use super::client_events;
use super::json_error_response;
use super::json_ok_response;
use super::orchestration;
use super::services;
use super::session;
use super::session_payload;
use super::shells;
use super::transcript;
use super::turn;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn route_request(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    auth_token: Option<&str>,
) -> HttpResponse {
    let background_shells = BackgroundShellManager::default();
    let event_log = new_event_log();
    route_request_with_manager_and_events(
        request,
        snapshot,
        command_queue,
        &background_shells,
        &event_log,
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
    let event_log = new_event_log();
    route_request_with_manager_and_events(
        request,
        snapshot,
        command_queue,
        background_shells,
        &event_log,
        auth_token,
    )
}

pub(crate) fn route_request_with_manager_and_events(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    background_shells: &BackgroundShellManager,
    event_log: &SharedEventLog,
    auth_token: Option<&str>,
) -> HttpResponse {
    if let Some(response) = authorize_request(request, auth_token) {
        return response;
    }

    route_authorized_request(
        request,
        snapshot,
        command_queue,
        background_shells,
        event_log,
    )
}

pub(in crate::local_api) fn authorize_request(
    request: &HttpRequest,
    auth_token: Option<&str>,
) -> Option<HttpResponse> {
    if request.path == "/healthz" && request.method == "GET" {
        return Some(json_ok_response(serde_json::json!({ "ok": true })));
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

pub(in crate::local_api) fn route_authorized_request(
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    background_shells: &BackgroundShellManager,
    event_log: &SharedEventLog,
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

    if request.method == "POST" && request.path == "/api/v1/session/client_event" {
        return client_events::handle_client_event_route(request, &current_snapshot, event_log);
    }

    if request.method == "POST" {
        if let Some(path) = request.path.strip_prefix("/api/v1/session/") {
            return route_session_scoped_post(
                path,
                request,
                &current_snapshot,
                command_queue,
                background_shells,
                event_log,
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
    event_log: &SharedEventLog,
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
        "client_event" => client_events::handle_session_client_event_route(
            request, snapshot, event_log, session_id,
        ),
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
