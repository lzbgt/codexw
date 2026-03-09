use super::super::super::*;

pub(in super::super::super) fn guidance_lines(state: &AppState) -> Option<Vec<String>> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let blocking_capability_issues = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues();

    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Missing)
    {
        return Some(vec![
            format!(
                "A blocking shell depends on missing service capability @{}.",
                issue.capability
            ),
            match unique_running_service_ref(state) {
                Some(job_ref) => format!(
                    "Start a service shell that provides the missing reusable role, or retarget the known running service with `:ps provide {job_ref} @{}` before waiting on the blocker.",
                    issue.capability
                ),
                None => format!(
                    "Start a service shell that provides the missing reusable role, or retarget an existing running service with `:ps provide <jobId|alias|n> @{}` before waiting on the blocker.",
                    issue.capability
                ),
            },
            format!(
                "Use :ps capabilities @{} and :ps dependencies missing @{} to inspect the provider map and blocked shell.",
                issue.capability, issue.capability
            ),
        ]);
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        return Some(vec![
            format!(
                "A blocking shell depends on ambiguous service capability @{}.",
                issue.capability
            ),
            "Resolve the conflicting reusable service role before relying on capability-based attachment.".to_string(),
            "Use :ps capabilities to inspect providers and consumers for that capability."
                .to_string(),
        ]);
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| format!("@{}", issue.capability));
        return Some(vec![
            format!(
                "A blocking shell is waiting on booting service capability @{}.",
                issue.capability
            ),
            format!(
                "Use :ps services booting @{} to inspect the provider and readiness state.",
                issue.capability
            ),
            format!(
                "Use :ps wait {provider_ref} [timeoutMs] when the booting service has a readiness contract."
            ),
        ]);
    }
    if prereqs > 0 {
        return Some(vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(prereqs, "prerequisite shell", "prerequisite shells")
            ),
            "Inspect :ps blockers to identify the gating job.".to_string(),
            "Use :ps wait <jobId|alias|@capability|n> [timeoutMs] for services with readiness contracts or :ps poll <jobId|alias|@capability|n> to inspect raw output.".to_string(),
        ]);
    }
    if waits > 0 {
        return Some(vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(waits, "agent wait", "agent waits")
            ),
            "Inspect :ps blockers to see the blocking agent dependencies.".to_string(),
            "Use :multi-agents to refresh or switch into the relevant agent thread.".to_string(),
        ]);
    }

    None
}

pub(in super::super::super) fn guidance_lines_for_tool(state: &AppState) -> Option<Vec<String>> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let blocking_capability_issues = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues();

    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Missing)
    {
        let blocker_ref = first_blocking_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        let service_ref =
            unique_running_service_ref(state).unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return Some(vec![
            format!(
                "A blocking shell depends on missing service capability @{}.",
                issue.capability
            ),
            format!(
                "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{}\"]}}` to retarget a running service or start a new provider for that reusable role.",
                issue.capability
            ),
            format!(
                "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` or `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{}\"}}` to inspect or retarget the blocked shell.",
                issue.capability
            ),
        ]);
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return Some(vec![
            format!(
                "A blocking shell depends on ambiguous service capability @{}.",
                issue.capability
            ),
            format!(
                "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role."
            ),
            format!(
                "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the conflicting providers and consumers.",
                issue.capability
            ),
        ]);
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| format!("@{}", issue.capability));
        return Some(vec![
            format!(
                "A blocking shell is waiting on booting service capability @{}.",
                issue.capability
            ),
            format!(
                "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{}\"}}` to inspect provider readiness.",
                issue.capability
            ),
            format!(
                "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` when later work depends on readiness."
            ),
        ]);
    }
    if prereqs > 0 {
        return Some(
            match unique_shell_ref_by_intent(state, BackgroundShellIntent::Prerequisite) {
                Some(job_ref) => vec![
                    format!(
                        "Main agent is blocked on {}.",
                        pluralize(prereqs, "prerequisite shell", "prerequisite shells")
                    ),
                    "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the gating shell."
                        .to_string(),
                    format!(
                        "Use `background_shell_poll {{\"jobId\":\"{job_ref}\"}}` to inspect the blocker output directly."
                    ),
                ],
                None => vec![
                    format!(
                        "Main agent is blocked on {}.",
                        pluralize(prereqs, "prerequisite shell", "prerequisite shells")
                    ),
                    "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the gating shells."
                        .to_string(),
                    "Use `background_shell_poll {\"jobId\":\"<jobId|alias|@capability>\"}` to inspect a blocker directly."
                        .to_string(),
                ],
            },
        );
    }
    if waits > 0 {
        return Some(vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(waits, "agent wait", "agent waits")
            ),
            "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the active wait dependencies.".to_string(),
            "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect cached and live agent workers.".to_string(),
        ]);
    }

    None
}
