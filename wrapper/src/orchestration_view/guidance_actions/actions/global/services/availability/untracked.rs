use super::super::super::super::super::*;

pub(in super::super::super::super::super) fn action_lines_for_untracked_services(
    state: &AppState,
    audience: ActionAudience,
) -> Option<Vec<String>> {
    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    if untracked_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    Some(match audience {
        ActionAudience::Operator => vec![
            "Run `:ps services untracked` to inspect reusable services that still lack readiness or attachment contract metadata."
                .to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Run `:ps contract {job_ref} <json-object>` to add fields such as `readyPattern`, `protocol`, `endpoint`, `attachHint`, or `recipes`."
                ),
                None => "Run `:ps contract <jobId|alias|@capability|n> <json-object>` to add fields such as `readyPattern`, `protocol`, `endpoint`, `attachHint`, or `recipes`."
                    .to_string(),
            },
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Run `:ps relabel {job_ref} <label|none>` if the service also needs a clearer operator-facing identity."
                ),
                None => "Run `:ps relabel <jobId|alias|@capability|n> <label|none>` if the service also needs a clearer operator-facing identity."
                    .to_string(),
            },
        ],
        ActionAudience::Tool => vec![
            "Use `background_shell_list_services {\"status\":\"untracked\"}` to inspect reusable services that still lack readiness or attachment contract metadata."
                .to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` to add reusable contract metadata in place."
                ),
                None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}` to add reusable contract metadata in place."
                    .to_string(),
            },
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"label\":\"service-label\"}}` if the service also needs a clearer operator-facing identity."
                ),
                None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"label\":\"service-label\"}` if the service also needs a clearer operator-facing identity."
                    .to_string(),
            },
        ],
    })
}
