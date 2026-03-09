use super::*;

#[path = "global/blockers.rs"]
mod blockers;
#[path = "global/services.rs"]
mod services;

pub(in super::super) fn guidance_lines(state: &AppState) -> Vec<String> {
    blockers::guidance_lines(state).unwrap_or_else(|| services::guidance_lines(state))
}

pub(in super::super) fn guidance_lines_for_tool(state: &AppState) -> Vec<String> {
    blockers::guidance_lines_for_tool(state)
        .unwrap_or_else(|| services::guidance_lines_for_tool(state))
}
