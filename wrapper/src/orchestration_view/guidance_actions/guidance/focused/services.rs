use super::super::super::shared::*;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::state::AppState;

pub(in super::super) fn guidance_lines_for_service_capability(
    state: &AppState,
    capability: &str,
) -> Result<Vec<String>, String> {
    Ok(
        match state
            .orchestration
            .background_shells
            .service_capability_issue_for_ref(capability)?
        {
            BackgroundShellCapabilityIssueClass::Missing => {
                vec![
                    format!("Reusable service capability @{capability} has no running provider."),
                    match unique_running_service_ref(state) {
                        Some(job_ref) => format!(
                            "Start a provider for @{capability} or retarget the known running service with `:ps provide {job_ref} @{capability}`."
                        ),
                        None => format!(
                            "Start a provider for @{capability} or retarget an existing running service with `:ps provide <jobId|alias|n> @{capability}`."
                        ),
                    },
                    format!(
                        "Use :ps capabilities @{capability} to confirm the missing-provider state."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Ambiguous => vec![
                format!("Reusable service capability @{capability} is ambiguous."),
                "Resolve the conflicting providers before relying on capability-based reuse."
                    .to_string(),
                format!("Use :ps capabilities @{capability} to inspect providers and consumers."),
            ],
            BackgroundShellCapabilityIssueClass::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!("Reusable service capability @{capability} is still booting."),
                    format!(
                        "Use :ps services booting @{capability} to inspect provider readiness."
                    ),
                    format!(
                        "Use :ps wait {provider_ref} 5000 when later work depends on readiness."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Untracked => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Reusable service capability @{capability} is provided by an untracked service."
                    ),
                    format!(
                        "Use :ps services untracked @{capability} to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use :ps contract {provider_ref} <json-object> to add readyPattern or attachment metadata in place."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Healthy => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!("Reusable service capability @{capability} is ready for reuse."),
                    format!(
                        "Use :ps attach {provider_ref} to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use {} to reuse it directly.",
                            operator_recipe_command(&provider_ref, recipe)
                        ),
                        None => format!(
                            "Use :ps attach {provider_ref} to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
        },
    )
}

pub(in super::super) fn guidance_lines_for_tool_service_capability(
    state: &AppState,
    capability: &str,
) -> Result<Vec<String>, String> {
    Ok(
        match state
            .orchestration
            .background_shells
            .service_capability_issue_for_ref(capability)?
        {
            BackgroundShellCapabilityIssueClass::Missing => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!("Reusable service capability @{capability} has no running provider."),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget a running service or start a new provider."
                    ),
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to confirm the missing-provider state."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Ambiguous => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!("Reusable service capability @{capability} is ambiguous."),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role."
                    ),
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!("Reusable service capability @{capability} is still booting."),
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
                    ),
                    format!(
                        "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` when later work depends on readiness."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Untracked => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Reusable service capability @{capability} is provided by an untracked service."
                    ),
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"untracked\",\"capability\":\"@{capability}\"}}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"label\":\"service-label\"}}` to add reusable contract metadata in place."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Healthy => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!("Reusable service capability @{capability} is ready for reuse."),
                    format!(
                        "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use `{}` to reuse it directly.",
                            tool_recipe_call(&provider_ref, recipe)
                        ),
                        None => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
        },
    )
}
