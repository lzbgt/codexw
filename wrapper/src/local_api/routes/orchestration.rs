use serde_json::json;

use crate::local_api::snapshot::LocalApiSnapshot;

use super::json_ok_response;

pub(super) fn handle_orchestration_status_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "orchestration": snapshot.orchestration_status,
    }))
}

pub(super) fn handle_orchestration_dependencies_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "dependencies": snapshot.orchestration_dependencies,
    }))
}

pub(super) fn handle_orchestration_workers_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "workers": snapshot.workers,
    }))
}
