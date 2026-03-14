use crate::state::AppState;
use std::time::Instant;

pub(crate) fn spinner_frame(started_at: Option<Instant>) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let elapsed_millis = started_at
        .map(|start| Instant::now().saturating_duration_since(start).as_millis())
        .unwrap_or(0);
    let frame_index = ((elapsed_millis / 80) as usize) % FRAMES.len();
    FRAMES[frame_index]
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
    if let Some((started_at, detail)) = render_async_tool_status(state) {
        return format!(
            "{} {} | {}",
            spinner_frame(Some(started_at)),
            detail,
            format_elapsed(Some(started_at))
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

fn render_async_tool_status(state: &AppState) -> Option<(Instant, String)> {
    if let Some(async_tool) = state.oldest_async_tool_activity() {
        let detail = if state.active_async_tool_requests.len() > 1 {
            format!(
                "async tools {}: {}",
                state.active_async_tool_requests.len(),
                async_tool.summary
            )
        } else {
            format!("async tool {}: {}", async_tool.tool, async_tool.summary)
        };
        let detail = if let Some(classification) = state.oldest_async_tool_supervision_class() {
            format!(
                "{} {detail} [{}]",
                classification.label(),
                classification.prompt_hint()
            )
        } else {
            detail
        };
        return Some((
            async_tool.started_at,
            append_async_backlog_suffix(state, detail),
        ));
    }
    let abandoned = state.oldest_abandoned_async_tool_request()?;
    let detail = if state.async_tool_backpressure_active() {
        format!(
            "async backlog saturated {}: {}",
            state.abandoned_async_tool_request_count(),
            abandoned.summary
        )
    } else {
        format!(
            "async backlog {}: {}",
            state.abandoned_async_tool_request_count(),
            abandoned.summary
        )
    };
    Some((abandoned.timed_out_at, detail))
}

fn append_async_backlog_suffix(state: &AppState, detail: String) -> String {
    let backlog = state.abandoned_async_tool_request_count();
    if backlog == 0 {
        return detail;
    }
    if state.async_tool_backpressure_active() {
        format!("{detail} [backlog saturated {backlog}]")
    } else {
        format!("{detail} [backlog {backlog}]")
    }
}

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    format!(
        "{} realtime | {}",
        spinner_frame(state.realtime_started_at),
        format_elapsed(state.realtime_started_at)
    )
}
