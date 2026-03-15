use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use serde_json::json;

use crate::local_api::LocalApiSnapshot;

use super::response::json_error_response_with_details;

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

fn attachment_has_active_conflicting_client(
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
