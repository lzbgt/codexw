#[path = "availability/booting.rs"]
mod booting;
#[path = "availability/ready.rs"]
mod ready;
#[path = "availability/untracked.rs"]
mod untracked;

use super::super::super::super::*;

pub(in super::super::super::super) fn action_lines_for_availability(
    state: &AppState,
    audience: ActionAudience,
) -> Option<Vec<String>> {
    ready::action_lines_for_ready_services(state, audience)
        .or_else(|| booting::action_lines_for_booting_services(state, audience))
        .or_else(|| untracked::action_lines_for_untracked_services(state, audience))
}
