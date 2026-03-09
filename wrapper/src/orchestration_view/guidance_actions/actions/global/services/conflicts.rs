use super::super::super::super::*;

pub(in super::super::super::super) fn action_lines_for_conflicts(
    state: &AppState,
    audience: ActionAudience,
) -> Option<Vec<String>> {
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    let (capability, _) = capability_conflicts.first()?;
    let provider_ref = first_provider_ref_for_capability(state, capability)
        .unwrap_or_else(|| "<jobId|alias|n>".to_string());
    Some(match audience {
        ActionAudience::Operator => vec![
            format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
            format!(
                "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
            ),
            format!("Run `:clean services @{capability}` to clear the ambiguous reusable role."),
            format!("Run `:ps services @{capability}` to verify the surviving providers."),
        ],
        ActionAudience::Tool => vec![
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
        ],
    })
}
