use crate::session_prompt_status::active_status_detail;
use crate::session_prompt_status::format_elapsed;
use crate::session_prompt_status::spinner_frame;
use crate::state::AppState;

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
