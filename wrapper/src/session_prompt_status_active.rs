#[path = "session_prompt_status_active/async_tools.rs"]
mod async_tools;
#[path = "session_prompt_status_active/timing.rs"]
mod timing;

use crate::state::AppState;
use std::time::Instant;

pub(crate) use timing::format_elapsed;
pub(crate) use timing::spinner_frame;

pub(crate) fn active_status_detail(state: &AppState) -> Option<&str> {
    state
        .last_status_line
        .as_deref()
        .filter(|line| !line.trim().is_empty() && *line != "ready")
}

pub(crate) fn render_exec_status(state: &AppState) -> String {
    if let Some(detail) = active_status_detail(state) {
        format!(
            "{} {} | {}",
            spinner_frame(state.activity_started_at),
            detail,
            format_elapsed(state.activity_started_at),
        )
    } else {
        format!(
            "{} cmd | {}",
            spinner_frame(state.activity_started_at),
            format_elapsed(state.activity_started_at),
        )
    }
}

pub(crate) fn render_turn_status(state: &AppState) -> String {
    if let Some((started_at, detail)) = async_tools::render_async_tool_status(state) {
        return format!(
            "{} {} | {}",
            spinner_frame(Some(started_at)),
            detail,
            format_elapsed(Some(started_at))
        );
    }
    if active_status_detail(state).is_none()
        && let Some(idle) = state.stalled_turn_idle_for()
    {
        let status = if state.has_active_server_command_activity() {
            "turn quiet; waiting on server command"
        } else {
            "turn quiet; awaiting app-server"
        };
        return format!(
            "{} {} {} | {}",
            spinner_frame(state.activity_started_at),
            status,
            format_elapsed(Some(Instant::now() - idle)),
            format_elapsed(state.activity_started_at)
        );
    }
    if let Some(detail) = active_status_detail(state) {
        format!(
            "{} {} | {}",
            spinner_frame(state.activity_started_at),
            detail,
            format_elapsed(state.activity_started_at),
        )
    } else {
        format!(
            "{} turn {} | {}",
            spinner_frame(state.activity_started_at),
            state.started_turn_count.max(1),
            format_elapsed(state.activity_started_at)
        )
    }
}

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    format!(
        "{} realtime | {}",
        spinner_frame(state.realtime_started_at),
        format_elapsed(state.realtime_started_at)
    )
}
