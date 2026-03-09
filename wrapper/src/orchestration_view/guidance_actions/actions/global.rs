use super::*;

#[path = "global/blockers.rs"]
mod blockers;
#[path = "global/services.rs"]
mod services;

pub(in super::super) fn action_lines(state: &AppState, audience: ActionAudience) -> Vec<String> {
    blockers::action_lines(state, audience)
        .unwrap_or_else(|| services::action_lines(state, audience))
}
