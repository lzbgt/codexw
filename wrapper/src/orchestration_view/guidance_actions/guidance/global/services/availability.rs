#[path = "availability/booting.rs"]
mod booting;
#[path = "availability/ready.rs"]
mod ready;
#[path = "availability/untracked.rs"]
mod untracked;

use super::super::super::super::*;

pub(in super::super::super::super) fn guidance_lines_for_availability(
    state: &AppState,
) -> Option<Vec<String>> {
    ready::guidance_lines_for_ready_services(state)
        .or_else(|| booting::guidance_lines_for_booting_services(state))
        .or_else(|| untracked::guidance_lines_for_untracked_services(state))
}

pub(in super::super::super::super) fn guidance_lines_for_availability_tool(
    state: &AppState,
) -> Option<Vec<String>> {
    ready::guidance_lines_for_ready_services_tool(state)
        .or_else(|| booting::guidance_lines_for_booting_services_tool(state))
        .or_else(|| untracked::guidance_lines_for_untracked_services_tool(state))
}
