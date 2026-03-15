use anyhow::Result;
use serde_json::json;

use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::runtime_event_sources::AsyncToolResponse;
use crate::state::AppState;
use crate::state::AsyncToolHealthCheck;
use crate::state::SupervisionNoticeTransition;

pub(super) fn handle_async_tool_response(
    tool_response: AsyncToolResponse,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<()> {
    if state.finish_async_tool_request(&tool_response.id).is_none() {
        if let Some(abandoned) = state.finish_abandoned_async_tool_request(&tool_response.id) {
            output.line_stderr(format!(
                "[tool] abandoned async tool worker finally returned after {}s: {}",
                abandoned.timed_out_elapsed().as_secs(),
                tool_response.summary
            ))?;
            return Ok(());
        }
        output.line_stderr(format!(
            "[tool] dropped late async tool response: {}",
            tool_response.summary
        ))?;
        return Ok(());
    }
    let _ = state.refresh_async_tool_supervision_notice();
    let success = tool_response
        .result
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    output.line_stderr(format!(
        "[tool] dynamic tool {}: {}",
        if success { "completed" } else { "failed" },
        tool_response.summary
    ))?;
    send_json(
        writer,
        &OutgoingResponse {
            id: tool_response.id,
            result: tool_response.result,
        },
    )?;
    Ok(())
}

pub(super) fn handle_supervision_tick(
    state: &mut AppState,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<()> {
    let resolved_cwd = std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    for expired in state.expire_timed_out_async_tool_requests() {
        let backlog = state.abandoned_async_tool_request_count();
        output.line_stderr(format!(
            "[self-supervision] forcing async tool failure after {}s (limit {}s, abandoned backlog {}): {}",
            expired.elapsed.as_secs(),
            expired.hard_timeout.as_secs(),
            backlog,
            expired.summary
        ))?;
        send_json(
            writer,
            &OutgoingResponse {
                id: expired.id,
                result: json!({
                    "contentItems": [{
                        "type": "inputText",
                        "text": format!(
                        "dynamic tool `{}` exceeded its {}s runtime limit and was failed locally to avoid hanging the active turn; summary: {}",
                            expired.tool,
                            expired.hard_timeout.as_secs(),
                            expired.summary
                        )
                    }],
                    "success": false
                }),
            },
        )?;
    }
    for check in state.collect_due_async_tool_health_checks() {
        output.line_stderr(format_async_tool_health_check_line(&check))?;
    }
    match state.refresh_async_tool_supervision_notice() {
        Some(SupervisionNoticeTransition::Raised(notice)) => {
            output.line_stderr(format_supervision_notice_line(
                &notice,
                state,
                &resolved_cwd,
            ))?;
        }
        Some(SupervisionNoticeTransition::Cleared) => {
            output.line_stderr("[self-supervision] async tool supervision cleared")?;
        }
        None => {}
    }
    Ok(())
}

pub(super) fn format_supervision_notice_line(
    notice: &crate::state::SupervisionNotice,
    state: &AppState,
    cwd: &str,
) -> String {
    let call = notice
        .source_call_id
        .as_deref()
        .map(|value| format!(" call={value}"))
        .unwrap_or_default();
    let target = match (
        notice.target_background_shell_reference.as_deref(),
        notice.target_background_shell_job_id.as_deref(),
    ) {
        (Some(reference), Some(job_id)) if reference != job_id => {
            format!(
                " target={} resolved={job_id}",
                crate::state::summarize_text(reference)
            )
        }
        (Some(reference), _) => format!(" target={}", crate::state::summarize_text(reference)),
        (None, Some(job_id)) => format!(" target={job_id}"),
        (None, None) => String::new(),
    };
    let observation = match notice.observed_background_shell_job.as_ref() {
        Some(job) => format!(
            " {}|{} job={} {}",
            notice.observation_state.label(),
            notice.output_state.label(),
            job.job_id,
            job.status
        ),
        None => format!(
            " {}|{}",
            notice.observation_state.label(),
            notice.output_state.label()
        ),
    };
    let options = crate::supervision_recovery::supervision_recovery_options(
        state,
        None,
        cwd,
        notice.classification,
    )
    .into_iter()
    .map(|option| {
        option
            .terminal_command
            .or(option.cli_command)
            .unwrap_or_else(|| option.kind.to_string())
    })
    .collect::<Vec<_>>()
    .join(",");
    format!(
        "[self-supervision] {} {} request={} worker={} owner={}{}{}{} [{}|{}|automation_ready={}] options={} {}",
        notice.classification.label(),
        notice.tool,
        notice.request_id,
        notice.worker_thread_name,
        notice.owner_kind.label(),
        call,
        target,
        observation,
        notice.recovery_policy_kind().label(),
        notice.recommended_action(),
        notice.automation_ready(),
        crate::state::summarize_text(&options),
        notice.summary
    )
}

pub(super) fn format_async_tool_health_check_line(check: &AsyncToolHealthCheck) -> String {
    let inspection = match check.supervision_classification {
        Some(classification) => format!(
            "{}|{}",
            classification.label(),
            classification.recommended_action()
        ),
        None => "monitoring".to_string(),
    };
    let call = check
        .source_call_id
        .as_deref()
        .map(|value| format!(" call={value}"))
        .unwrap_or_default();
    let target = match (
        check.target_background_shell_reference.as_deref(),
        check.target_background_shell_job_id.as_deref(),
    ) {
        (Some(reference), Some(job_id)) if reference != job_id => {
            format!(
                " target={} resolved={job_id}",
                crate::state::summarize_text(reference)
            )
        }
        (Some(reference), _) => format!(" target={}", crate::state::summarize_text(reference)),
        (None, Some(job_id)) => format!(" target={job_id}"),
        (None, None) => String::new(),
    };
    let observation = match check.observed_background_shell_job.as_ref() {
        Some(job) => {
            let output_age = job
                .last_output_age
                .map(|age| format!(" output_age={}s", age.as_secs()))
                .unwrap_or_default();
            let output = job
                .latest_output_preview()
                .map(|line| format!(" output={}", crate::state::summarize_text(line)))
                .unwrap_or_default();
            format!(
                "{}|{} {} via {}{}{} job={} {} lines={} command={}{}{}",
                check.observation_state.label(),
                check.output_state.label(),
                check.owner_kind.label(),
                check.worker_thread_name,
                call,
                target,
                job.job_id,
                job.status,
                job.total_lines,
                crate::state::summarize_text(&job.command),
                output_age,
                output
            )
        }
        None => format!(
            "{}|{} {} via {}{}{}",
            check.observation_state.label(),
            check.output_state.label(),
            check.owner_kind.label(),
            check.worker_thread_name,
            call,
            target
        ),
    };
    format!(
        "[self-supervision] async worker check {}s [{}] {} next={}s for {}: {}",
        check.elapsed.as_secs(),
        inspection,
        observation,
        check.next_health_check_in.as_secs(),
        check.tool,
        check.summary
    )
}
