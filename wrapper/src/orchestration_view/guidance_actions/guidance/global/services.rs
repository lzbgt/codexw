#[path = "services/availability.rs"]
mod availability;
#[path = "services/conflicts.rs"]
mod conflicts;
#[path = "services/residuals.rs"]
mod residuals;

use super::super::super::*;

pub(in super::super::super) fn guidance_lines(state: &AppState) -> Vec<String> {
    conflicts::guidance_lines_for_conflicts(state)
        .or_else(|| availability::guidance_lines_for_availability(state))
        .or_else(|| residuals::guidance_lines_for_residuals(state))
        .unwrap_or_default()
}

#[cfg(test)]
pub(in super::super::super) fn guidance_lines_for_tool(state: &AppState) -> Vec<String> {
    conflicts::guidance_lines_for_conflicts_tool(state)
        .or_else(|| availability::guidance_lines_for_availability_tool(state))
        .or_else(|| residuals::guidance_lines_for_residuals_tool(state))
        .unwrap_or_default()
}
