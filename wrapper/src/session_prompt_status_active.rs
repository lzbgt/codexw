use crate::state::AppState;
use std::time::Instant;

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

pub(crate) fn render_exec_status(state: &AppState) -> String {
    if let Some(detail) = active_status_detail(state) {
        format!(
            "{} {} · {}",
            spinner_frame(state.activity_started_at),
            detail,
            format_elapsed(state.activity_started_at),
        )
    } else {
        format!(
            "{} cmd · {}",
            spinner_frame(state.activity_started_at),
            format_elapsed(state.activity_started_at),
        )
    }
}

pub(crate) fn render_turn_status(state: &AppState) -> String {
    if let Some(detail) = active_status_detail(state) {
        format!(
            "{} {} · {}",
            spinner_frame(state.activity_started_at),
            detail,
            format_elapsed(state.activity_started_at),
        )
    } else {
        format!(
            "{} turn {} · {}",
            spinner_frame(state.activity_started_at),
            state.started_turn_count.max(1),
            format_elapsed(state.activity_started_at)
        )
    }
}

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    format!(
        "{} realtime · {}",
        spinner_frame(state.realtime_started_at),
        format_elapsed(state.realtime_started_at)
    )
}
