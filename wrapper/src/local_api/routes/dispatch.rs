#[path = "dispatch/auth.rs"]
mod auth;
#[path = "dispatch/scoped_get.rs"]
mod scoped_get;
#[path = "dispatch/scoped_post.rs"]
mod scoped_post;

use crate::background_shells::BackgroundShellManager;
use crate::local_api::SharedCommandQueue;
use crate::local_api::SharedEventLog;
use crate::local_api::SharedSnapshot;
use crate::local_api::new_event_log;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;

use super::client_events;
use super::json_error_response;
use super::json_ok_response;
use super::session;
use super::session_payload;
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
    auth::authorize_request(request, auth_token)
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
            return scoped_post::route_session_scoped_post(
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
        return scoped_get::route_session_scoped_get(path, &current_snapshot);
    }

    json_error_response(404, "not_found", "unknown route")
}
