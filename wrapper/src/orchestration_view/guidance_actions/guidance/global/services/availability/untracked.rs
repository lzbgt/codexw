use super::super::super::super::super::*;

pub(in super::super::super::super::super) fn guidance_lines_for_untracked_services(
    state: &AppState,
) -> Option<Vec<String>> {
    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    if untracked_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    Some(vec![
        format!(
            "{} missing readiness or attachment metadata.",
            pluralize(untracked_services, "service shell is", "service shells are")
        ),
        "Use :ps services untracked to inspect services that still need contract metadata."
            .to_string(),
        match provider_ref.as_deref() {
            Some(job_ref) => format!(
                "Use :ps contract {job_ref} <json-object> or :ps relabel {job_ref} <label|none> to make the service reusable in place."
            ),
            None => "Use :ps contract <jobId|alias|@capability|n> <json-object> or :ps relabel <jobId|alias|@capability|n> <label|none> to make the service reusable in place."
                .to_string(),
        },
    ])
}

#[cfg(test)]
pub(in super::super::super::super::super) fn guidance_lines_for_untracked_services_tool(
    state: &AppState,
) -> Option<Vec<String>> {
    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    if untracked_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    Some(vec![
        format!(
            "{} missing readiness or attachment metadata.",
            pluralize(untracked_services, "service shell is", "service shells are")
        ),
        "Use `background_shell_list_services {\"status\":\"untracked\"}` to inspect services that still need contract metadata.".to_string(),
        match provider_ref.as_deref() {
            Some(job_ref) => format!(
                "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` or `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"label\":\"service-label\"}}` to make the service reusable in place."
            ),
            None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}` or `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"label\":\"service-label\"}` to make the service reusable in place.".to_string(),
        },
    ])
}
