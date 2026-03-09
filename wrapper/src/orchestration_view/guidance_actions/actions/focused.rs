use super::*;

#[path = "focused/blockers.rs"]
mod blockers;
#[path = "focused/services.rs"]
mod services;

pub(in super::super) fn action_lines_for_capability(
    state: &AppState,
    audience: ActionAudience,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(lines) = blockers::action_lines_for_blocking_capability(state, audience, capability)
    {
        return Ok(lines);
    }
    Ok(services::action_lines_for_service_capability(
        state, audience, capability,
    )?)
}
