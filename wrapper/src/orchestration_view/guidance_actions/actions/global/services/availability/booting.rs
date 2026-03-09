use super::super::super::super::super::*;

pub(in super::super::super::super::super) fn action_lines_for_booting_services(
    state: &AppState,
    audience: ActionAudience,
) -> Option<Vec<String>> {
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    if booting_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    Some(match audience {
        ActionAudience::Operator => vec![
            "Run `:ps services booting` to inspect readiness state and startup metadata."
                .to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => {
                    format!("Run `:ps wait {job_ref} [timeoutMs]` for the booting service you need.")
                }
                None => "Run `:ps wait <jobId|alias|@capability|n> [timeoutMs]` for the booting service you need."
                    .to_string(),
            },
            "Run `:ps capabilities booting` to keep the capability view focused.".to_string(),
        ],
        ActionAudience::Tool => vec![
            "Use `background_shell_list_services {\"status\":\"booting\"}` to inspect readiness state and startup metadata.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{job_ref}\",\"timeoutMs\":5000}}` for the booting service you need."
                ),
                None => "Use `background_shell_wait_ready {\"jobId\":\"<jobId|alias|@capability>\",\"timeoutMs\":5000}` for the booting service you need.".to_string(),
            },
            "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
        ],
    })
}
