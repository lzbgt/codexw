use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use super::super::SharedSnapshot;

pub(super) fn apply_attachment_metadata(
    snapshot: &SharedSnapshot,
    client_id: Option<&str>,
    lease_seconds: Option<u64>,
) {
    let Ok(mut guard) = snapshot.write() else {
        return;
    };
    guard.attachment_client_id = client_id.map(ToOwned::to_owned);
    guard.attachment_lease_seconds = lease_seconds;
    guard.attachment_lease_expires_at_ms = lease_seconds.and_then(lease_expiry_ms);
}

pub(super) fn clear_attachment_metadata(snapshot: &SharedSnapshot, client_id: Option<&str>) {
    let Ok(mut guard) = snapshot.write() else {
        return;
    };
    if client_id.is_some() && guard.attachment_client_id.as_deref() != client_id {
        return;
    }
    guard.attachment_client_id = None;
    guard.attachment_lease_seconds = None;
    guard.attachment_lease_expires_at_ms = None;
}

fn lease_expiry_ms(seconds: u64) -> Option<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let delta = u128::from(seconds).checked_mul(1000)?;
    let expiry = now.checked_add(delta)?;
    u64::try_from(expiry).ok()
}
