use serde_json::json;

use crate::local_api::snapshot::LocalApiSnapshot;

pub(super) fn runtime_payload(snapshot: &LocalApiSnapshot) -> serde_json::Value {
    json!({
        "ok": true,
        "runtime": snapshot.runtime,
        "session_id": snapshot.session_id,
        "cwd": snapshot.cwd,
    })
}
