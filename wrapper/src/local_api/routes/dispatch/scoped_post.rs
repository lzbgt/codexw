use crate::background_shells::BackgroundShellManager;
use crate::local_api::LocalApiSnapshot;
use crate::local_api::SharedCommandQueue;
use crate::local_api::SharedEventLog;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::HttpResponse;

use super::super::client_events;
use super::super::json_error_response;
use super::super::services;
use super::super::session;
use super::super::shells;
use super::super::turn;

pub(super) fn route_session_scoped_post(
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
