use serde_json::json;

use crate::local_api::snapshot::LocalApiSnapshot;

use super::json_ok_response;

pub(super) fn handle_transcript_route(
    snapshot: &LocalApiSnapshot,
) -> crate::local_api::server::HttpResponse {
    json_ok_response(json!({
        "ok": true,
        "session_id": snapshot.session_id,
        "transcript": snapshot.transcript,
    }))
}
