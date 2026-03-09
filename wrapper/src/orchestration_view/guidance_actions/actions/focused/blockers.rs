use super::super::super::shared::*;
use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::orchestration_view::guidance_actions::ActionAudience;
use crate::state::AppState;

pub(in super::super) fn action_lines_for_blocking_capability(
    state: &AppState,
    audience: ActionAudience,
    capability: &str,
) -> Option<Vec<String>> {
    let issue = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)?;

    Some(match (issue.status, audience) {
        (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Operator) => {
            let blocker_ref = first_blocking_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            let service_ref =
                unique_running_service_ref(state).unwrap_or_else(|| "<jobId|alias|n>".to_string());
            vec![
                format!(
                    "Run `:ps capabilities @{capability}` to inspect the missing provider map."
                ),
                format!(
                    "Run `:ps provide {service_ref} @{capability}` to retarget an existing running service, or start a new provider for that role."
                ),
                format!(
                    "Run `:ps depend {blocker_ref} <@capability...|none>` to retarget the blocked shell if it should wait on a different reusable role."
                ),
                format!(
                    "Run `:ps dependencies missing @{capability}` to inspect the blocked dependency edges."
                ),
                format!(
                    "If the blocked shell is no longer needed, run `:clean blockers @{capability}`."
                ),
            ]
        }
        (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Tool) => {
            let blocker_ref = first_blocking_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            let service_ref =
                unique_running_service_ref(state).unwrap_or_else(|| "<jobId|alias|n>".to_string());
            vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the missing provider map."
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget an existing running service, or start a new provider for that capability."
                ),
                format!(
                    "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` to retarget the blocked shell if it should depend on a different reusable role."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{capability}\"}}` to inspect the blocked dependency edges."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"blockers\",\"capability\":\"@{capability}\"}}` to abandon the blocking prerequisite shells if they are no longer needed."
                ),
            ]
        }
        (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Operator) => {
            let blocker_ref = first_blocking_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            let provider_ref = first_provider_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            vec![
                format!(
                    "Run `:ps capabilities @{capability}` to inspect the ambiguous provider set."
                ),
                format!(
                    "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                ),
                format!(
                    "Run `:ps depend {blocker_ref} <@capability...|none>` if the blocked shell should be retargeted to a different dependency role."
                ),
                format!(
                    "Run `:clean services @{capability}` to clear the conflicting reusable role in one step."
                ),
                format!("Run `:ps services @{capability}` to inspect the remaining providers."),
            ]
        }
        (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Tool) => {
            let blocker_ref = first_blocking_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            let provider_ref = first_provider_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the ambiguous provider set."
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                ),
                format!(
                    "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` if the blocked shell should be retargeted to a different dependency role."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the conflicting reusable role in one step."
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to inspect the remaining providers."
                ),
            ]
        }
        (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Operator) => {
            let provider_ref = first_provider_ref_for_capability(state, capability)
                .unwrap_or_else(|| format!("@{capability}"));
            vec![
                format!(
                    "Run `:ps services booting @{capability}` to inspect the booting provider state."
                ),
                format!("Run `:ps wait {provider_ref} 5000` to wait on the capability provider."),
                format!(
                    "Run `:ps dependencies booting @{capability}` to keep the dependency view focused."
                ),
            ]
        }
        (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Tool) => {
            let provider_ref = first_provider_ref_for_capability(state, capability)
                .unwrap_or_else(|| format!("@{capability}"));
            vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect the booting provider state."
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` to wait on the capability provider."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"booting\",\"capability\":\"@{capability}\"}}` to keep the dependency view focused."
                ),
            ]
        }
        (BackgroundShellCapabilityDependencyState::Satisfied, _) => vec![],
    })
}
