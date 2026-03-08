use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::state::AppState;

use crate::collaboration_preset::CollaborationModePreset;
use crate::collaboration_preset::find_collaboration_mode_by_selector;

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
    state
        .active_collaboration_mode
        .as_ref()
        .map(CollaborationModePreset::summary)
        .unwrap_or_else(|| "default".to_string())
}

pub(crate) fn current_collaboration_mode_label(state: &AppState) -> Option<String> {
    let preset = state.active_collaboration_mode.as_ref()?;
    if preset.is_plan() {
        Some("plan mode".to_string())
    } else {
        Some(format!("collab {}", preset.name))
    }
}

pub(crate) fn render_collaboration_modes(state: &AppState) -> String {
    let current = summarize_active_collaboration_mode(state);
    if state.collaboration_modes.is_empty() {
        return format!(
            "current         {current}\nno collaboration mode presets available from app-server"
        );
    }

    let mut lines = vec![
        format!("current         {current}"),
        "available presets".to_string(),
    ];
    for (index, preset) in state.collaboration_modes.iter().enumerate() {
        lines.push(format!(" {:>2}. {}", index + 1, preset.summary()));
    }
    lines.push("Use /collab <name|mode> or /plan to switch.".to_string());
    lines.join("\n")
}

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
