use serde_json::json;

use crate::local_api::snapshot::LocalApiSnapshot;

use super::super::json_error_response;
use super::super::json_ok_response;
use super::super::resolve_shell_snapshot;

pub(super) fn handle_shells_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "shells": snapshot.workers.background_shells,
    }))
}

pub(super) fn handle_shell_detail_route(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "shell": shell,
        })),
        Err((code, message)) => json_error_response(404, code, message),
    }
}

pub(super) fn handle_shell_poll_route(
    snapshot: &LocalApiSnapshot,
    reference: &str,
) -> crate::local_api::server::HttpResponse {
    match resolve_shell_snapshot(snapshot, reference) {
        Ok(shell) => json_ok_response(json!({
            "ok": true,
            "session_id": snapshot.session_id,
            "interaction": {
                "kind": "shell.poll",
                "shell_ref": reference,
            },
            "shell": shell,
        })),
        Err((code, message)) => json_error_response(404, code, message),
    }
}
