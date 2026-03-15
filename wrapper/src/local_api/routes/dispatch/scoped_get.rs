use crate::local_api::LocalApiSnapshot;
use crate::local_api::server::HttpResponse;

use super::super::json_error_response;
use super::super::json_ok_response;
use super::super::orchestration;
use super::super::services;
use super::super::session_payload;
use super::super::shells;
use super::super::transcript;

pub(super) fn route_session_scoped_get(path: &str, snapshot: &LocalApiSnapshot) -> HttpResponse {
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
