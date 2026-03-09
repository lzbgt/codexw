use super::super::super::super::super::*;

pub(in super::super::super::super::super) fn guidance_lines_for_booting_services(
    state: &AppState,
) -> Option<Vec<String>> {
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    if booting_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    Some(vec![
        format!(
            "{} still booting.",
            pluralize(booting_services, "service shell is", "service shells are")
        ),
        "Use :ps services to inspect readiness state and startup metadata.".to_string(),
        match provider_ref.as_deref() {
            Some(job_ref) => format!(
                "Use :ps wait {job_ref} [timeoutMs] when later work depends on service readiness."
            ),
            None => "Use :ps wait <jobId|alias|@capability|n> [timeoutMs] when later work depends on service readiness."
                .to_string(),
        },
    ])
}

pub(in super::super::super::super::super) fn guidance_lines_for_booting_services_tool(
    state: &AppState,
) -> Option<Vec<String>> {
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    if booting_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    Some(vec![
        format!(
            "{} still booting.",
            pluralize(booting_services, "service shell is", "service shells are")
        ),
        "Use `background_shell_list_services {\"status\":\"booting\"}` to inspect readiness state and startup metadata.".to_string(),
        match provider_ref.as_deref() {
            Some(job_ref) => format!(
                "Use `background_shell_wait_ready {{\"jobId\":\"{job_ref}\",\"timeoutMs\":5000}}` when later work depends on service readiness."
            ),
            None => "Use `background_shell_wait_ready {\"jobId\":\"<jobId|alias|@capability>\",\"timeoutMs\":5000}` when later work depends on service readiness.".to_string(),
        },
    ])
}
