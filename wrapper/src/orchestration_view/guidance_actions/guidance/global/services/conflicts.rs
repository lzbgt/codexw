use super::super::super::super::*;

pub(in super::super::super::super) fn guidance_lines_for_conflicts(
    state: &AppState,
) -> Option<Vec<String>> {
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    if capability_conflicts.is_empty() {
        return None;
    }
    let conflict_count = capability_conflicts.len();
    let first = &capability_conflicts[0].0;
    Some(vec![
        format!(
            "{} detected across service capabilities.",
            pluralize(conflict_count, "capability conflict is", "capability conflicts are")
        ),
        format!("Resolve ambiguous reuse targets such as @{first} before relying on capability-based attachment."),
        "Use :ps capabilities to inspect the ambiguous capability map and assign more specific capabilities.".to_string(),
    ])
}

#[cfg(test)]
pub(in super::super::super::super) fn guidance_lines_for_conflicts_tool(
    state: &AppState,
) -> Option<Vec<String>> {
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    if capability_conflicts.is_empty() {
        return None;
    }
    let conflict_count = capability_conflicts.len();
    let first = &capability_conflicts[0].0;
    Some(vec![
        format!(
            "{} detected across service capabilities.",
            pluralize(
                conflict_count,
                "capability conflict is",
                "capability conflicts are"
            )
        ),
        format!(
            "Use `background_shell_update_service {{\"jobId\":\"<jobId|alias|n>\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"<jobId|alias|n>\",\"capabilities\":null}}` to resolve ambiguous reuse targets such as @{first}."
        ),
        format!(
            "Use `background_shell_inspect_capability {{\"capability\":\"@{first}\"}}` to inspect the ambiguous capability map."
        ),
    ])
}
