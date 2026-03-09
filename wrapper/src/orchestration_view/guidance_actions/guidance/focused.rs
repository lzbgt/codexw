use super::*;

#[path = "focused/blockers.rs"]
mod blockers;
#[path = "focused/services.rs"]
mod services;

pub(in super::super) fn guidance_lines_for_capability(
    state: &AppState,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(lines) = blockers::guidance_lines_for_blocking_capability(state, capability) {
        return Ok(lines);
    }
    Ok(services::guidance_lines_for_service_capability(
        state, capability,
    )?)
}

pub(in super::super) fn guidance_lines_for_tool_capability(
    state: &AppState,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(lines) = blockers::guidance_lines_for_tool_blocking_capability(state, capability) {
        return Ok(lines);
    }
    Ok(services::guidance_lines_for_tool_service_capability(
        state, capability,
    )?)
}
