use anyhow::Result;

use crate::collaboration_preset::CollaborationModePreset;
use crate::collaboration_preset::find_collaboration_mode_by_selector;
use crate::output::Output;
use crate::state::AppState;

use super::CollaborationModeAction;
use super::collaboration_view::render_collaboration_modes;

pub(crate) fn apply_collaboration_mode_action(
    state: &mut AppState,
    action: CollaborationModeAction,
    output: &mut Output,
) -> Result<()> {
    match action {
        CollaborationModeAction::CacheOnly => {}
        CollaborationModeAction::ShowList => {
            output.block_stdout("Collaboration modes", &render_collaboration_modes(state))?;
        }
        CollaborationModeAction::TogglePlan => {
            if state
                .active_collaboration_mode
                .as_ref()
                .is_some_and(CollaborationModePreset::is_plan)
            {
                state.active_collaboration_mode = None;
                output.line_stderr("[session] collaboration mode cleared; using default mode")?;
            } else if let Some(plan) = state
                .collaboration_modes
                .iter()
                .find(|preset| preset.is_plan())
                .cloned()
            {
                let summary = plan.summary();
                state.active_collaboration_mode = Some(plan);
                output.line_stderr(format!("[session] switched to {summary}"))?;
            } else {
                output.line_stderr(
                    "[session] no plan collaboration preset is available from app-server",
                )?;
            }
        }
        CollaborationModeAction::SetMode(selector) => {
            let normalized = selector.trim().to_ascii_lowercase();
            if matches!(normalized.as_str(), "default" | "off" | "none" | "clear") {
                state.active_collaboration_mode = None;
                output.line_stderr("[session] collaboration mode cleared; using default mode")?;
            } else if let Some(preset) =
                find_collaboration_mode_by_selector(&state.collaboration_modes, &normalized)
            {
                let summary = preset.summary();
                state.active_collaboration_mode = Some(preset);
                output.line_stderr(format!("[session] switched to {summary}"))?;
            } else if state.collaboration_modes.is_empty() {
                output.line_stderr(
                    "[session] no collaboration mode presets are available from app-server",
                )?;
            } else {
                output.line_stderr(format!("[session] unknown collaboration mode: {selector}"))?;
                output.block_stdout("Collaboration modes", &render_collaboration_modes(state))?;
            }
        }
    }

    Ok(())
}
