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
    if let Some((request_id, async_tool)) = state.oldest_async_tool_entry() {
        let observation = state.async_tool_observation(async_tool);
        let prompt_cwd = prompt_render_cwd();
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
        let recovery_options = state
            .oldest_async_tool_supervision_class()
            .map(|classification| prompt_recovery_options(state, &prompt_cwd, classification))
            .filter(|options| !options.is_empty())
            .map(|options| format!("; opts {}", options.join("/")))
            .unwrap_or_default();
        let observation_detail = match observation.observed_background_shell_job.as_ref() {
            Some(job) => {
                let last_output = job
                    .latest_output_preview()
                    .map(|line| format!("; out {}", summarize_inline(line)))
                    .unwrap_or_default();
                let command_detail = format!("; cmd {}", summarize_inline(&job.command));
                let output_state = format_output_state_detail(observation.output_state, job);
                format!(
                    "{}; {}; job {} {}{}{}",
                    observation.observation_state.prompt_label(),
                    output_state,
                    job.job_id,
                    job.status,
                    command_detail,
                    last_output
                )
            }
            None => format!(
                "{}; {}",
                observation.observation_state.prompt_label(),
                observation.output_state.prompt_label()
            ),
        };
        let target_detail = match (
            async_tool.target_background_shell_reference.as_deref(),
            async_tool.target_background_shell_job_id.as_deref(),
        ) {
            (Some(reference), Some(job_id)) if reference != job_id => {
                format!("; target {}->{}", summarize_inline(reference), job_id)
            }
            (Some(reference), _) => format!("; target {}", summarize_inline(reference)),
            (None, Some(job_id)) => format!("; target {job_id}"),
            (None, None) => String::new(),
        };
        let request_detail = format!(
            "; req {}",
            summarize_inline(&crate::state::request_id_label(request_id))
        );
        let source_detail = async_tool
            .source_call_id
            .as_deref()
            .map(|call_id| format!("; call {}", summarize_inline(call_id)))
            .unwrap_or_default();
        let worker_detail = format!(
            "; worker {}",
            summarize_inline(&async_tool.worker_thread_name)
        );
        let detail = format!(
            "{detail} [{}; {}{}{}{}{}; next check {}{}]",
            observation.owner_kind.prompt_label(),
            observation_detail,
            request_detail,
            source_detail,
            worker_detail,
            target_detail,
            format_elapsed(Some(Instant::now() - async_tool.next_health_check_in())),
            recovery_options
        );
        return Some((
            async_tool.started_at,
            append_async_backlog_suffix(state, detail),
        ));
    }
    let (request_id, abandoned) = state.oldest_abandoned_async_tool_entry()?;
    let prompt_cwd = prompt_render_cwd();
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
    let request_detail = format!(
        "; req {}",
        summarize_inline(&crate::state::request_id_label(request_id))
    );
    let worker_detail = format!(
        "; worker {}",
        summarize_inline(&abandoned.worker_thread_name)
    );
    let source_detail = abandoned
        .source_call_id
        .as_deref()
        .map(|call_id| format!("; call {}", summarize_inline(call_id)))
        .unwrap_or_default();
    let target_detail = match (
        abandoned.target_background_shell_reference.as_deref(),
        abandoned.target_background_shell_job_id.as_deref(),
    ) {
        (Some(reference), Some(job_id)) if reference != job_id => {
            format!("; target {}->{}", summarize_inline(reference), job_id)
        }
        (Some(reference), _) => format!("; target {}", summarize_inline(reference)),
        (None, Some(job_id)) => format!("; target {job_id}"),
        (None, None) => String::new(),
    };
    let observation = state.abandoned_async_tool_observation(abandoned);
    let observation_detail = match observation.observed_background_shell_job.as_ref() {
        Some(job) => {
            let last_output = job
                .latest_output_preview()
                .map(|line| format!("; out {}", summarize_inline(line)))
                .unwrap_or_default();
            let command_detail = format!("; cmd {}", summarize_inline(&job.command));
            let output_state = format_output_state_detail(observation.output_state, job);
            format!(
                " [{}; {}; job {} {}{}{}]",
                observation.observation_state.prompt_label(),
                output_state,
                job.job_id,
                job.status,
                command_detail,
                last_output
            )
        }
        None => format!(
            " [{}; {}]",
            observation.observation_state.prompt_label(),
            observation.output_state.prompt_label()
        ),
    };
    let recovery_options =
        crate::supervision_recovery::async_backpressure_recovery_options(state, None, &prompt_cwd)
            .into_iter()
            .map(|option| match option.kind {
                "observe_status" => ":status",
                "interrupt_turn" => ":interrupt",
                "exit_and_resume" => "resume",
                _ => option.kind,
            })
            .collect::<Vec<_>>();
    let recovery_detail = if recovery_options.is_empty() {
        String::new()
    } else {
        format!("; opts {}", recovery_options.join("/"))
    };
    Some((
        abandoned.timed_out_at,
        format!(
            "{detail}{request_detail}{worker_detail}{source_detail}{target_detail}{observation_detail}{recovery_detail}"
        ),
    ))
}

fn format_output_state_detail(
    output_state: crate::state::AsyncToolOutputState,
    job: &crate::state::AsyncToolObservedBackgroundShellJob,
) -> String {
    match (output_state, job.last_output_age) {
        (crate::state::AsyncToolOutputState::RecentOutputObserved, Some(age)) => {
            format!("{} {}", output_state.prompt_label(), summarize_age(age))
        }
        (crate::state::AsyncToolOutputState::StaleOutputObserved, Some(age)) => {
            format!("{} {}", output_state.prompt_label(), summarize_age(age))
        }
        _ => output_state.prompt_label().to_string(),
    }
}

fn summarize_age(age: std::time::Duration) -> String {
    let secs = age.as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else {
        format!("{}m{:02}s ago", secs / 60, secs % 60)
    }
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

fn prompt_recovery_options(
    state: &AppState,
    cwd: &str,
    classification: crate::state::AsyncToolSupervisionClass,
) -> Vec<&'static str> {
    crate::supervision_recovery::supervision_recovery_options(state, None, cwd, classification)
        .into_iter()
        .map(|option| match option.kind {
            "observe_status" => ":status",
            "interrupt_turn" => ":interrupt",
            "exit_and_resume" => "resume",
            _ => option.kind,
        })
        .collect()
}

fn prompt_render_cwd() -> String {
    std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    format!(
        "{} realtime | {}",
        spinner_frame(state.realtime_started_at),
        format_elapsed(state.realtime_started_at)
    )
}

fn summarize_inline(text: &str) -> String {
    const MAX_CHARS: usize = 48;
    let mut chars = text.chars();
    let summary = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{summary}...")
    } else {
        summary
    }
}
