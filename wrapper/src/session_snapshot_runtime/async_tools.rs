use crate::session_prompt_status_active::format_elapsed;
use crate::state::AppState;
use crate::state::summarize_text;

pub(super) fn push_async_runtime_lines(
    lines: &mut Vec<String>,
    state: &AppState,
    status_cwd: &str,
) {
    push_active_async_tool_lines(lines, state);
    push_async_worker_lines(lines, state);
    push_abandoned_async_tool_lines(lines, state, status_cwd);
}

fn push_active_async_tool_lines(lines: &mut Vec<String>, state: &AppState) {
    let Some((request_id, async_tool)) = state.oldest_async_tool_entry() else {
        return;
    };
    let observation = state.async_tool_observation(async_tool);
    lines.push(format!(
        "async tools     {}",
        state.active_async_tool_requests.len()
    ));
    if let Some(classification) = state.oldest_async_tool_supervision_class() {
        lines.push(format!("async class     {}", classification.label()));
        lines.push(format!(
            "async action    {}",
            classification.recommended_action()
        ));
    }
    lines.push(format!(
        "async tool      {}",
        summarize_text(&async_tool.summary)
    ));
    lines.push(format!(
        "async req       {}",
        crate::state::request_id_label(request_id)
    ));
    lines.push(format!(
        "async thread    {}",
        summarize_text(&async_tool.worker_thread_name)
    ));
    lines.push(format!(
        "async owner     {}",
        observation.owner_kind.label()
    ));
    if let Some(source_call_id) = async_tool.source_call_id.as_deref() {
        lines.push(format!("async call      {source_call_id}"));
    }
    if let Some(target_reference) = async_tool.target_background_shell_reference.as_deref() {
        lines.push(format!("async target    {target_reference}"));
    }
    if let Some(target_job_id) = async_tool.target_background_shell_job_id.as_deref() {
        lines.push(format!("async target jb {target_job_id}"));
    }
    lines.push(format!(
        "async obs       {}",
        observation.observation_state.label()
    ));
    lines.push(format!(
        "async out       {}",
        observation.output_state.label()
    ));
    if let Some(job) = observation.observed_background_shell_job.as_ref() {
        lines.push(format!("async job       {} {}", job.job_id, job.status));
        lines.push(format!("async cmd       {}", summarize_text(&job.command)));
        lines.push(format!("async lines     {}", job.total_lines));
        if let Some(age) = job.last_output_age {
            lines.push(format!(
                "async out age   {}",
                format_elapsed(Some(std::time::Instant::now() - age))
            ));
        }
        if let Some(output) = job.latest_output_preview() {
            lines.push(format!("async output    {}", summarize_text(output)));
        }
    }
    lines.push(format!(
        "async chk in    {}",
        format_elapsed(Some(
            std::time::Instant::now() - async_tool.next_health_check_in()
        ))
    ));
    lines.push(format!(
        "async time      {}",
        format_elapsed(Some(async_tool.started_at))
    ));
}

fn push_async_worker_lines(lines: &mut Vec<String>, state: &AppState) {
    let worker_statuses = state.async_tool_worker_statuses();
    let Some(worker) = worker_statuses.first() else {
        return;
    };
    lines.push(format!(
        "async worker    {} {}",
        worker.lifecycle_state.label(),
        summarize_text(&worker.worker_thread_name)
    ));
    lines.push(format!("async worker id {}", worker.request_id));
    lines.push(format!("async worker ow {}", worker.owner_kind.label()));
    if let Some(source_call_id) = worker.source_call_id.as_deref() {
        lines.push(format!("async worker cl {source_call_id}"));
    }
    if let Some(target_reference) = worker.target_background_shell_reference.as_deref() {
        lines.push(format!("async worker tr {target_reference}"));
    }
    if let Some(target_job_id) = worker.target_background_shell_job_id.as_deref() {
        lines.push(format!("async worker tj {target_job_id}"));
    }
    if let Some(observation_state) = worker.observation_state {
        lines.push(format!("async worker ob {}", observation_state.label()));
    }
    if let Some(output_state) = worker.output_state {
        lines.push(format!("async worker os {}", output_state.label()));
    }
    if let Some(job) = worker.observed_background_shell_job.as_ref() {
        lines.push(format!("async worker jb {} {}", job.job_id, job.status));
        lines.push(format!("async worker ln {}", job.total_lines));
        if let Some(age) = job.last_output_age {
            lines.push(format!(
                "async worker oa {}",
                format_elapsed(Some(std::time::Instant::now() - age))
            ));
        }
        if let Some(output) = job.latest_output_preview() {
            lines.push(format!("async worker ot {}", summarize_text(output)));
        }
    }
    if let Some(next_health_check_in) = worker.next_health_check_in {
        lines.push(format!(
            "async worker ck {}",
            format_elapsed(Some(std::time::Instant::now() - next_health_check_in))
        ));
    }
}

fn push_abandoned_async_tool_lines(lines: &mut Vec<String>, state: &AppState, status_cwd: &str) {
    let Some((request_id, abandoned)) = state.oldest_abandoned_async_tool_entry() else {
        return;
    };
    let observation = state.abandoned_async_tool_observation(abandoned);
    lines.push(format!(
        "async aban      {}",
        state.abandoned_async_tool_request_count()
    ));
    lines.push(format!(
        "async stale rq  {}",
        crate::state::request_id_label(request_id)
    ));
    lines.push(format!("async stale wk  {}", abandoned.worker_thread_name));
    lines.push(format!(
        "async stale     {}",
        summarize_text(&abandoned.summary)
    ));
    lines.push(format!(
        "async stale tm  {}",
        format_elapsed(Some(abandoned.timed_out_at))
    ));
    if let Some(source_call_id) = abandoned.source_call_id.as_deref() {
        lines.push(format!("async stale cl  {source_call_id}"));
    }
    if let Some(target_reference) = abandoned.target_background_shell_reference.as_deref() {
        lines.push(format!("async stale tr  {target_reference}"));
    }
    if let Some(target_job_id) = abandoned.target_background_shell_job_id.as_deref() {
        lines.push(format!("async stale tj  {target_job_id}"));
    }
    lines.push(format!(
        "async stale ob  {}",
        observation.observation_state.label()
    ));
    lines.push(format!(
        "async stale os  {}",
        observation.output_state.label()
    ));
    if let Some(job) = observation.observed_background_shell_job.as_ref() {
        lines.push(format!("async stale jb  {} {}", job.job_id, job.status));
        lines.push(format!("async stale ln  {}", job.total_lines));
        if let Some(age) = job.last_output_age {
            lines.push(format!(
                "async stale oa  {}",
                format_elapsed(Some(std::time::Instant::now() - age))
            ));
        }
        if let Some(output) = job.latest_output_preview() {
            lines.push(format!("async stale ot  {}", summarize_text(output)));
        }
    }
    lines.push(format!(
        "async guard     {}",
        if state.async_tool_backpressure_active() {
            "saturated"
        } else {
            "monitoring"
        }
    ));
    lines.push(format!(
        "async guard act {}",
        crate::supervision_recovery::async_backpressure_recommended_action(state)
    ));
    lines.push(format!(
        "async guard pol {}",
        crate::supervision_recovery::async_backpressure_recovery_policy_kind(state).label()
    ));
    lines.push(format!(
        "async guard auto {}",
        crate::supervision_recovery::async_backpressure_automation_ready(state)
    ));
    for option in
        crate::supervision_recovery::async_backpressure_recovery_options(state, None, status_cwd)
    {
        let command = option
            .terminal_command
            .or(option.cli_command)
            .unwrap_or_else(|| option.label.to_string());
        lines.push(format!(
            "async guard opt {} {}",
            option.kind,
            summarize_text(&command)
        ));
    }
}
