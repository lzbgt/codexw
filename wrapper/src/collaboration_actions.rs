use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::state::AppState;

#[path = "collaboration_apply.rs"]
mod collaboration_apply;
#[path = "collaboration_view.rs"]
mod collaboration_view;

use crate::collaboration_preset::CollaborationModePreset;

#[derive(Debug, Clone)]
pub(crate) enum CollaborationModeAction {
    CacheOnly,
    ShowList,
    TogglePlan,
    SetMode(String),
}

pub(crate) fn current_collaboration_mode_value(state: &AppState) -> Option<Value> {
    state
        .active_collaboration_mode
        .as_ref()
        .and_then(CollaborationModePreset::turn_start_value)
}

pub(crate) fn summarize_active_collaboration_mode(state: &AppState) -> String {
    collaboration_view::summarize_active_collaboration_mode(state)
}

pub(crate) fn current_collaboration_mode_label(state: &AppState) -> Option<String> {
    collaboration_view::current_collaboration_mode_label(state)
}

#[cfg(test)]
pub(crate) fn render_collaboration_modes(state: &AppState) -> String {
    collaboration_view::render_collaboration_modes(state)
}

pub(crate) fn apply_collaboration_mode_action(
    state: &mut AppState,
    action: CollaborationModeAction,
    output: &mut Output,
) -> Result<()> {
    collaboration_apply::apply_collaboration_mode_action(state, action, output)
}
