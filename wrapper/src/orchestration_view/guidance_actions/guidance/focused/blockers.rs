use super::super::super::shared::*;
use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::state::AppState;

pub(in super::super) fn guidance_lines_for_blocking_capability(
    state: &AppState,
    capability: &str,
) -> Option<Vec<String>> {
    let issue = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)?;

    Some(match issue.status {
        BackgroundShellCapabilityDependencyState::Missing => vec![
            format!("A blocking shell depends on missing service capability @{capability}."),
            match unique_running_service_ref(state) {
                Some(job_ref) => format!(
                    "Start a provider for @{capability} or retarget the known running service with `:ps provide {job_ref} @{capability}` before waiting on the shell result."
                ),
                None => format!(
                    "Start a provider for @{capability} or retarget an existing running service with `:ps provide <jobId|alias|n> @{capability}` before waiting on the shell result."
                ),
            },
            format!(
                "Use :ps capabilities @{capability} and :ps dependencies missing @{capability} to inspect the exact blocker."
            ),
        ],
        BackgroundShellCapabilityDependencyState::Ambiguous => vec![
            format!("A blocking shell depends on ambiguous service capability @{capability}."),
            "Resolve the conflicting reusable service role before relying on capability-based attachment.".to_string(),
            format!(
                "Use :ps capabilities @{capability} and :ps services @{capability} to inspect the conflicting providers."
            ),
        ],
        BackgroundShellCapabilityDependencyState::Booting => {
            let provider_ref = first_provider_ref_for_capability(state, capability)
                .unwrap_or_else(|| format!("@{capability}"));
            vec![
                format!("A blocking shell is waiting on booting service capability @{capability}."),
                format!(
                    "Use :ps services booting @{capability} to inspect the provider and readiness state."
                ),
                format!(
                    "Use :ps wait {provider_ref} 5000 when later work depends on readiness."
                ),
            ]
        }
        BackgroundShellCapabilityDependencyState::Satisfied => vec![],
    })
}

pub(in super::super) fn guidance_lines_for_tool_blocking_capability(
    state: &AppState,
    capability: &str,
) -> Option<Vec<String>> {
    let issue = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)?;

    Some(match issue.status {
        BackgroundShellCapabilityDependencyState::Missing => {
            let blocker_ref = first_blocking_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            let service_ref =
                unique_running_service_ref(state).unwrap_or_else(|| "<jobId|alias|n>".to_string());
            vec![
                format!("A blocking shell depends on missing service capability @{capability}."),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget a running service or start a new provider for that reusable role."
                ),
                format!(
                    "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` or `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{capability}\"}}` to inspect or retarget the blocked shell."
                ),
            ]
        }
        BackgroundShellCapabilityDependencyState::Ambiguous => {
            let provider_ref = first_provider_ref_for_capability(state, capability)
                .unwrap_or_else(|| "<jobId|alias|n>".to_string());
            vec![
                format!("A blocking shell depends on ambiguous service capability @{capability}."),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role."
                ),
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the conflicting providers and consumers."
                ),
            ]
        }
        BackgroundShellCapabilityDependencyState::Booting => {
            let provider_ref = first_provider_ref_for_capability(state, capability)
                .unwrap_or_else(|| format!("@{capability}"));
            vec![
                format!("A blocking shell is waiting on booting service capability @{capability}."),
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` when later work depends on readiness."
                ),
            ]
        }
        BackgroundShellCapabilityDependencyState::Satisfied => vec![],
    })
}
