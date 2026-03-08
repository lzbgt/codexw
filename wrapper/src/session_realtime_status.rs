use crate::session_prompt_status_active::format_elapsed;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    let mut lines = vec![format!("active          {}", state.realtime_active)];
    lines.push(format!(
        "session         {}",
        state.realtime_session_id.as_deref().unwrap_or("-")
    ));
    lines.push(format!(
        "prompt          {}",
        summarize_text(state.realtime_prompt.as_deref().unwrap_or("-"))
    ));
    if state.realtime_active {
        lines.push(format!(
            "active time     {}",
            format_elapsed(state.realtime_started_at)
        ));
    }
    if let Some(error) = state.realtime_last_error.as_deref() {
        lines.push(format!("last error      {}", summarize_text(error)));
    }
    lines.push(
        "commands        /realtime start [prompt...] | /realtime send <text> | /realtime stop"
            .to_string(),
    );
    lines.push("audio           output audio deltas are not rendered in codexw".to_string());
    lines.join("\n")
}
