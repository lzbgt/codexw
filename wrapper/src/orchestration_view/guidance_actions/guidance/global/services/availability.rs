use super::super::super::super::*;

pub(in super::super::super::super) fn guidance_lines_for_availability(
    state: &AppState,
) -> Option<Vec<String>> {
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return Some(vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
            "Use :ps services to inspect attachment metadata and available recipes.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => match recipe.as_ref() {
                    Some(recipe) => format!(
                        "Use :ps attach {job_ref} or {} to reuse the service directly.",
                        operator_recipe_command(job_ref, recipe)
                    ),
                    None => format!(
                        "Use :ps attach {job_ref} to inspect endpoint and recipe details for the ready service."
                    ),
                },
                None => "Use :ps attach <jobId|alias|@capability|n> or :ps run <jobId|alias|@capability|n> <recipe> [json-args] to reuse the service directly."
                    .to_string(),
            },
        ]);
    }

    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return Some(vec![
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
        ]);
    }

    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return Some(vec![
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
        ]);
    }

    None
}

pub(in super::super::super::super) fn guidance_lines_for_availability_tool(
    state: &AppState,
) -> Option<Vec<String>> {
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return Some(vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
            "Use `background_shell_list_services {\"status\":\"ready\"}` to inspect attachment metadata and available recipes.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => match recipe.as_ref() {
                    Some(recipe) => format!(
                        "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` or `{}` to reuse the ready service directly.",
                        tool_recipe_call(job_ref, recipe)
                    ),
                    None => format!(
                        "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` to inspect endpoint and recipe details for the ready service."
                    ),
                },
                None => "Use `background_shell_attach {\"jobId\":\"<jobId|alias|@capability>\"}` or `background_shell_invoke_recipe {\"jobId\":\"<jobId|alias|@capability>\",\"recipe\":\"...\"}` to reuse the ready service directly.".to_string(),
            },
        ]);
    }

    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return Some(vec![
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
        ]);
    }

    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return Some(vec![
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
        ]);
    }

    None
}
