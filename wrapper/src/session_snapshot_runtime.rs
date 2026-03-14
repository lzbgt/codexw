use crate::Cli;
use crate::background_terminals::background_terminal_count;
use crate::orchestration_view::orchestration_background_summary;
use crate::orchestration_view::orchestration_next_action_summary;
use crate::orchestration_view::orchestration_runtime_summary;
use crate::session_prompt_status_active::format_elapsed;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::status_account::render_account_summary;
use crate::status_rate_windows::render_rate_limit_lines;
use crate::status_token_usage::render_token_usage_summary;

pub(crate) fn render_status_runtime(cli: &Cli, state: &AppState) -> Vec<String> {
    let status_cwd = status_render_cwd(cli);
    let mut lines = Vec::new();

    if state.realtime_active || state.realtime_session_id.is_some() {
        lines.push(format!(
            "realtime id     {}",
            state.realtime_session_id.as_deref().unwrap_or("-")
        ));
    }
    if state.realtime_active {
        lines.push(format!(
            "realtime time   {}",
            format_elapsed(state.realtime_started_at)
        ));
    }
    if let Some(prompt) = state.realtime_prompt.as_deref() {
        lines.push(format!("realtime prompt {}", summarize_text(prompt)));
    }
    if let Some(error) = state.realtime_last_error.as_deref() {
        lines.push(format!("realtime error  {}", summarize_text(error)));
    }
    if background_terminal_count(state) > 0 {
        lines.push(format!(
            "background      {}",
            background_terminal_count(state)
        ));
    }
    if let Some(summary) = orchestration_background_summary(state) {
        lines.push(format!("background cls  {summary}"));
    }
    if let Some(summary) = orchestration_runtime_summary(state) {
        lines.push(format!("workers         {summary}"));
    }
    if let Some(next_action) = orchestration_next_action_summary(state) {
        lines.push(format!("next action     {next_action}"));
    }

    if let Some(account) = render_account_summary(state.account_info.as_ref()) {
        lines.push(format!("account         {account}"));
    }
    if state.turn_running || state.active_exec_process_id.is_some() {
        lines.push(format!(
            "active time     {}",
            format_elapsed(state.activity_started_at)
        ));
    }
    if let Some((request_id, async_tool)) = state.oldest_async_tool_entry() {
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
    if let Some(worker) = state.async_tool_worker_statuses().first() {
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
    if let Some((request_id, abandoned)) = state.oldest_abandoned_async_tool_entry() {
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
        for option in crate::supervision_recovery::async_backpressure_recovery_options(
            state,
            None,
            &status_cwd,
        ) {
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
    if let Some(notice) = state
        .active_supervision_notice
        .clone()
        .or_else(|| state.current_async_tool_supervision_notice())
    {
        lines.extend(render_supervision_notice_runtime_lines(
            &notice,
            state,
            &status_cwd,
        ));
    }
    lines.extend(render_rate_limit_lines(state.rate_limits.as_ref()));
    if let Some(token_usage) = render_token_usage_summary(state.last_token_usage.as_ref()) {
        lines.push(format!("tokens          {token_usage}"));
    }
    if let Some(last_status) = state.last_status_line.as_deref() {
        lines.push(format!("status          {last_status}"));
    }
    if let Some(last_message) = state.last_agent_message.as_deref() {
        lines.push(format!("last reply      {}", summarize_text(last_message)));
    }
    if let Some(diff) = state.last_turn_diff.as_deref() {
        lines.push(format!("diff            {} chars", diff.chars().count()));
    }

    lines
}

fn render_supervision_notice_runtime_lines(
    notice: &crate::state::SupervisionNotice,
    state: &AppState,
    cwd: &str,
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "supervision     {} {}",
            notice.classification.label(),
            notice.tool
        ),
        format!("supervision pol {}", notice.recovery_policy_kind().label()),
        format!("supervision act {}", notice.recommended_action()),
        format!("supervision auto {}", notice.automation_ready()),
        format!("supervision req {}", notice.request_id),
        format!(
            "supervision th  {}",
            summarize_text(&notice.worker_thread_name)
        ),
        format!("supervision ow  {}", notice.owner_kind.label()),
        format!("supervision sum {}", summarize_text(&notice.summary)),
        format!("supervision ob  {}", notice.observation_state.label()),
        format!("supervision os  {}", notice.output_state.label()),
    ];
    if let Some(source_call_id) = notice.source_call_id.as_deref() {
        lines.push(format!("supervision cl  {source_call_id}"));
    }
    if let Some(target_reference) = notice.target_background_shell_reference.as_deref() {
        lines.push(format!("supervision tr  {target_reference}"));
    }
    if let Some(target_job_id) = notice.target_background_shell_job_id.as_deref() {
        lines.push(format!("supervision tj  {target_job_id}"));
    }
    if let Some(job) = notice.observed_background_shell_job.as_ref() {
        lines.push(format!("supervision jb  {} {}", job.job_id, job.status));
        lines.push(format!("supervision cmd {}", summarize_text(&job.command)));
        lines.push(format!("supervision ln  {}", job.total_lines));
        if let Some(age) = job.last_output_age {
            lines.push(format!(
                "supervision oa  {}",
                format_elapsed(Some(std::time::Instant::now() - age))
            ));
        }
        if let Some(output) = job.latest_output_preview() {
            lines.push(format!("supervision ot  {}", summarize_text(output)));
        }
    }
    for option in crate::supervision_recovery::supervision_recovery_options(
        state,
        None,
        cwd,
        notice.classification,
    ) {
        let command = option
            .terminal_command
            .or(option.cli_command)
            .unwrap_or_else(|| option.label.to_string());
        lines.push(format!(
            "supervision opt {} {}",
            option.kind,
            summarize_text(&command)
        ));
    }
    lines
}

fn status_render_cwd(cli: &Cli) -> String {
    cli.cwd.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| ".".to_string())
    })
}
