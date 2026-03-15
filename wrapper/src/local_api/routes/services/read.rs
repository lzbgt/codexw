use serde_json::json;

use crate::local_api::server::HttpResponse;
use crate::local_api::snapshot::LocalApiSnapshot;

use super::super::json_error_response;
use super::super::json_ok_response;
use super::super::resolve_shell_snapshot;

pub(super) fn handle_services_route(snapshot: &LocalApiSnapshot) -> HttpResponse {
    let services: Vec<_> = snapshot
        .workers
        .background_shells
        .iter()
        .filter(|shell| shell.intent == "service")
        .cloned()
        .collect();
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "services": services,
    }))
}

pub(super) fn handle_service_detail_route(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> HttpResponse {
    let shell = match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => shell,
        Err((code, message)) => return json_error_response(404, code, message),
    };
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "service": shell,
    }))
}

pub(super) fn handle_capabilities_route(snapshot: &LocalApiSnapshot) -> HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "capabilities": snapshot.capabilities,
    }))
}

pub(super) fn handle_capability_detail_route(
    snapshot: &LocalApiSnapshot,
    capability: &str,
) -> HttpResponse {
    let Some(entry) = snapshot
        .capabilities
        .iter()
        .find(|entry| entry.capability == capability)
    else {
        return json_error_response(404, "capability_not_found", "unknown capability reference");
    };
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "capability": entry,
    }))
}
