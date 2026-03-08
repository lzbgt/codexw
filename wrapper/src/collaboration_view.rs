use crate::state::AppState;

use crate::collaboration_preset::CollaborationModePreset;

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
