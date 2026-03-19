use super::super::super::super::super::*;

pub(in super::super::super::super::super) fn guidance_lines_for_ready_services(
    state: &AppState,
) -> Option<Vec<String>> {
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    if ready_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let recipe =
        unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    Some(vec![
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
    ])
}

#[cfg(test)]
pub(in super::super::super::super::super) fn guidance_lines_for_ready_services_tool(
    state: &AppState,
) -> Option<Vec<String>> {
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    if ready_services == 0 {
        return None;
    }
    let provider_ref =
        unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let recipe =
        unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    Some(vec![
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
    ])
}
