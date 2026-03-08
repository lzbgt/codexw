use crate::background_terminals::background_terminal_status_suffix;
use crate::collaboration_view::current_collaboration_mode_label;
use crate::model_personality_view::personality_label;
use crate::state::AppState;

pub(crate) fn render_ready_status(state: &AppState) -> String {
    let base = match current_collaboration_mode_label(state) {
        Some(label) => match state.active_personality.as_deref() {
            Some(personality) => format!(
                "ready | {label} | {} | {} turns",
                personality_label(personality),
                state.completed_turn_count
            ),
            None => format!("ready | {label} | {} turns", state.completed_turn_count),
        },
        None => match state.active_personality.as_deref() {
            Some(personality) => format!(
                "ready | {} | {} turns",
                personality_label(personality),
                state.completed_turn_count
            ),
            None => format!("ready | {} turns", state.completed_turn_count),
        },
    };
    if let Some(background) = background_terminal_status_suffix(state) {
        format!("{base} | {background}")
    } else {
        base
    }
}
