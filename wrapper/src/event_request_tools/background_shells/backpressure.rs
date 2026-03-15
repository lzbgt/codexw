use crate::state::AbandonedAsyncToolRequest;
use crate::state::AppState;
use crate::state::AsyncToolObservedBackgroundShellJob;

pub(super) fn background_shell_backpressure_failure(
    tool: &str,
    backlog: usize,
    oldest_summary: &str,
    oldest_context: &str,
    backpressure: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "contentItems": [{
            "type": "inputText",
            "text": format!(
                "dynamic tool `{tool}` was refused locally because {backlog} timed-out background-shell worker(s) are still unresolved; newest work is blocked until the abandoned backlog drains or the operator interrupts/exits and resumes the thread; oldest backlog summary: {oldest_summary}{oldest_context}"
            )
        }],
        "failure_kind": "async_tool_backpressure",
        "backpressure": backpressure,
        "success": false
    })
}

pub(super) fn background_shell_backpressure_details(
    resolved_cwd: &str,
    state: &AppState,
    request: Option<(&crate::rpc::RequestId, &AbandonedAsyncToolRequest)>,
) -> serde_json::Value {
    let recovery_options = async_backpressure_recovery_options_json(
        state.realtime_session_id.as_deref(),
        resolved_cwd,
        state,
    );
    match request {
        Some((request_id, request)) => {
            let observation = state.abandoned_async_tool_observation(request);
            serde_json::json!({
                "abandoned_request_count": state.abandoned_async_tool_request_count(),
                "saturation_threshold": crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
                "saturated": state.async_tool_backpressure_active(),
                "recommended_action": crate::supervision_recovery::async_backpressure_recommended_action(state),
                "recovery_policy": {
                    "kind": crate::supervision_recovery::async_backpressure_recovery_policy_kind(state).label(),
                    "automation_ready": crate::supervision_recovery::async_backpressure_automation_ready(state),
                },
                "recovery_options": recovery_options,
                "oldest_request_id": crate::state::request_id_label(request_id),
                "oldest_thread_name": request.worker_thread_name.as_str(),
                "oldest_tool": request.tool.as_str(),
                "oldest_summary": request.summary.as_str(),
                "oldest_source_call_id": request.source_call_id.as_deref(),
                "oldest_target_background_shell_reference": request.target_background_shell_reference.as_deref(),
                "oldest_target_background_shell_job_id": request.target_background_shell_job_id.as_deref(),
                "oldest_observation_state": observation.observation_state.label(),
                "oldest_output_state": observation.output_state.label(),
                "oldest_observed_background_shell_job": observation
                    .observed_background_shell_job
                    .as_ref()
                    .map(backpressure_observed_background_shell_job),
                "oldest_elapsed_before_timeout_seconds": request.elapsed_before_timeout.as_secs(),
                "oldest_hard_timeout_seconds": request.hard_timeout.as_secs(),
                "oldest_elapsed_seconds": request.timed_out_elapsed().as_secs()
            })
        }
        None => serde_json::json!({
            "abandoned_request_count": state.abandoned_async_tool_request_count(),
            "saturation_threshold": crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
            "saturated": state.async_tool_backpressure_active(),
            "recommended_action": crate::supervision_recovery::async_backpressure_recommended_action(state),
            "recovery_policy": {
                "kind": crate::supervision_recovery::async_backpressure_recovery_policy_kind(state).label(),
                "automation_ready": crate::supervision_recovery::async_backpressure_automation_ready(state),
            },
            "recovery_options": recovery_options,
            "oldest_request_id": serde_json::Value::Null,
            "oldest_thread_name": serde_json::Value::Null,
            "oldest_tool": serde_json::Value::Null,
            "oldest_summary": serde_json::Value::Null,
            "oldest_source_call_id": serde_json::Value::Null,
            "oldest_target_background_shell_reference": serde_json::Value::Null,
            "oldest_target_background_shell_job_id": serde_json::Value::Null,
            "oldest_observation_state": serde_json::Value::Null,
            "oldest_output_state": serde_json::Value::Null,
            "oldest_observed_background_shell_job": serde_json::Value::Null,
            "oldest_elapsed_before_timeout_seconds": serde_json::Value::Null,
            "oldest_hard_timeout_seconds": serde_json::Value::Null,
            "oldest_elapsed_seconds": serde_json::Value::Null
        }),
    }
}

pub(super) fn summarize_abandoned_backpressure_context(
    state: &AppState,
    request: &AbandonedAsyncToolRequest,
) -> String {
    let observation = state.abandoned_async_tool_observation(request);
    let call = request
        .source_call_id
        .as_deref()
        .map(|value| format!(" call={value}"))
        .unwrap_or_default();
    let target = match (
        request.target_background_shell_reference.as_deref(),
        request.target_background_shell_job_id.as_deref(),
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
    match observation.observed_background_shell_job.as_ref() {
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
                " [{}|{}{}{} job={} {} lines={} command={}{}{}]",
                observation.observation_state.label(),
                observation.output_state.label(),
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
            " [{}|{}{}{}]",
            observation.observation_state.label(),
            observation.output_state.label(),
            call,
            target
        ),
    }
}

fn async_backpressure_recovery_options_json(
    session_id: Option<&str>,
    resolved_cwd: &str,
    state: &AppState,
) -> serde_json::Value {
    serde_json::Value::Array(
        crate::supervision_recovery::async_backpressure_recovery_options(
            state,
            session_id,
            resolved_cwd,
        )
        .into_iter()
        .map(|option| {
            serde_json::json!({
                "kind": option.kind,
                "label": option.label,
                "automation_ready": option.automation_ready,
                "cli_command": option.cli_command,
                "local_api_method": option.local_api_method,
                "local_api_path": option.local_api_path,
            })
        })
        .collect(),
    )
}

fn backpressure_observed_background_shell_job(
    job: &AsyncToolObservedBackgroundShellJob,
) -> serde_json::Value {
    serde_json::json!({
        "job_id": job.job_id.as_str(),
        "status": job.status.as_str(),
        "command": job.command.as_str(),
        "total_lines": job.total_lines,
        "last_output_age_seconds": job.last_output_age.map(|value| value.as_secs()),
        "recent_lines": job.recent_lines
    })
}
