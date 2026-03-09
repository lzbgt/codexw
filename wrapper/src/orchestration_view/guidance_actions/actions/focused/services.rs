use super::super::super::shared::*;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::orchestration_view::guidance_actions::ActionAudience;
use crate::state::AppState;

pub(in super::super) fn action_lines_for_service_capability(
    state: &AppState,
    audience: ActionAudience,
    capability: &str,
) -> Result<Vec<String>, String> {
    Ok(
        match (
            state
                .orchestration
                .background_shells
                .service_capability_issue_for_ref(capability)?,
            audience,
        ) {
            (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Operator) => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Run `:ps capabilities @{capability}` to confirm there is no running provider."
                    ),
                    format!(
                        "Run `:ps provide {service_ref} @{capability}` to retarget an existing running service, or start a new provider for that role."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Tool) => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to confirm there is no running provider."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget an existing running service, or start a new provider for that capability."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Operator) => vec![
                format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
                {
                    let provider_ref = first_provider_ref_for_capability(state, capability)
                        .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                    format!(
                        "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                    )
                },
                format!(
                    "Run `:clean services @{capability}` to clear the ambiguous reusable role."
                ),
                format!("Run `:ps services @{capability}` to verify the surviving providers."),
            ],
            (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                    ),
                    format!(
                        "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the ambiguous reusable role."
                    ),
                    format!(
                        "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to verify the surviving providers."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services booting @{capability}` to inspect provider readiness."
                    ),
                    format!("Run `:ps wait {provider_ref} 5000` for the booting service you need."),
                    "Run `:ps capabilities booting` to keep the capability view focused."
                        .to_string(),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` for the booting service you need."
                ),
                "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
            ]
            }
            (BackgroundShellCapabilityIssueClass::Untracked, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services untracked @{capability}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Run `:ps contract {provider_ref} <json-object>` to add `readyPattern`, `protocol`, `endpoint`, or recipes in place for @{capability}."
                    ),
                    format!(
                        "Run `:ps relabel {provider_ref} <label|none>` if the reusable service needs a clearer operator label."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Untracked, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"untracked\",\"capability\":\"@{capability}\"}}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` to add a live readiness or attachment contract for @{capability}."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"label\":\"service-label\"}}` if the reusable service needs a clearer operator label."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!(
                        "Run `:ps attach {provider_ref}` to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Run `:ps attach {provider_ref}` or `{}` to reuse the ready service directly.",
                            operator_recipe_command(&provider_ref, recipe)
                        ),
                        None => format!(
                            "Run `:ps attach {provider_ref}` to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
            (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!(
                        "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` or `{}` to reuse the ready service directly.",
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
