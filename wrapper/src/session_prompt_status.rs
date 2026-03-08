use std::time::Instant;

use crate::state::AppState;

#[path = "session_prompt_status_active.rs"]
mod session_prompt_status_active;
#[path = "session_prompt_status_ready.rs"]
mod session_prompt_status_ready;

pub(crate) fn render_prompt_status(state: &AppState) -> String {
    if state.active_exec_process_id.is_some() {
        session_prompt_status_active::render_exec_status(state)
    } else if state.turn_running {
        session_prompt_status_active::render_turn_status(state)
    } else if state.realtime_active {
        session_prompt_status_active::render_realtime_status(state)
    } else {
        session_prompt_status_ready::render_ready_status(state)
    }
}

pub(crate) fn spinner_frame(started_at: Option<Instant>) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let idx = started_at
        .map(|start| {
            ((Instant::now().saturating_duration_since(start).as_millis() / 100) as usize)
                % FRAMES.len()
        })
        .unwrap_or(0);
    FRAMES[idx]
}

pub(crate) fn format_elapsed(started_at: Option<Instant>) -> String {
    let elapsed = started_at
        .map(|start| Instant::now().saturating_duration_since(start).as_secs())
        .unwrap_or(0);
    if elapsed < 60 {
        format!("{elapsed}s")
    } else {
        format!("{}m{:02}s", elapsed / 60, elapsed % 60)
    }
}

pub(crate) fn active_status_detail(state: &AppState) -> Option<&str> {
    state
        .last_status_line
        .as_deref()
        .filter(|line| !line.trim().is_empty() && *line != "ready")
}

pub(crate) fn personality_label(personality: &str) -> &str {
    match personality {
        "none" => "None",
        "friendly" => "Friendly",
        "pragmatic" => "Pragmatic",
        _ => personality,
    }
}

pub(crate) fn current_collaboration_label(state: &AppState) -> Option<String> {
    crate::collaboration_actions::current_collaboration_mode_label(state)
}
