#[path = "services/availability.rs"]
mod availability;
#[path = "services/conflicts.rs"]
mod conflicts;
#[path = "services/residuals.rs"]
mod residuals;

use super::super::super::*;

pub(in super::super::super) fn action_lines(
    state: &AppState,
    audience: ActionAudience,
) -> Vec<String> {
    conflicts::action_lines_for_conflicts(state, audience)
        .or_else(|| availability::action_lines_for_availability(state, audience))
        .or_else(|| residuals::action_lines_for_residuals(state, audience))
        .unwrap_or_default()
}
