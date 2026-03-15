use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::background_shells::BackgroundShellManager;
use crate::client_dynamic_tools::execute_background_shell_tool_call_with_manager;
use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_event_sources::AsyncToolResponse;
use crate::state::AppState;
use crate::transcript_approval_summary::summarize_tool_request;

const DEFAULT_BACKGROUND_SHELL_TOOL_TIMEOUT_MS: u64 = 30_000;
const BACKGROUND_SHELL_START_TIMEOUT_MS: u64 = 15_000;
const BACKGROUND_SHELL_REQUEST_TIMEOUT_GRACE_MS: u64 = 5_000;
const MAX_BACKGROUND_SHELL_TOOL_TIMEOUT_MS: u64 = 300_000;

pub(super) fn handle_background_shell_tool_request(
    request: &RpcRequest,
    tool: &str,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<()> {
    if state.async_tool_backpressure_active() {
        let backlog = state.abandoned_async_tool_request_count();
        let oldest = state.oldest_abandoned_async_tool_entry();
        let oldest_summary = oldest
            .map(|(_, request)| request.summary.as_str())
            .unwrap_or("background-shell async backlog saturated");
        let oldest_context = oldest
            .map(|(_, request)| summarize_abandoned_backpressure_context(state, request))
            .unwrap_or_default();
        let recovery_policy =
            crate::supervision_recovery::async_backpressure_recovery_policy_kind(state).label();
        let recommended_action =
            crate::supervision_recovery::async_backpressure_recommended_action(state);
        let recovery_options = crate::supervision_recovery::async_backpressure_recovery_options(
            state,
            state.realtime_session_id.as_deref(),
            resolved_cwd,
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
        output.line_stderr(format!(
            "[self-supervision] refusing async tool while abandoned backlog is saturated ({backlog}) [{recovery_policy}|{recommended_action}|automation_ready=false] options={} : {}{}",
            crate::state::summarize_text(&recovery_options),
            oldest_summary,
            oldest_context
        ))?;
        send_json(
            writer,
            &OutgoingResponse {
                id: request.id.clone(),
                result: background_shell_backpressure_failure(
                    tool,
                    backlog,
                    oldest_summary,
                    &oldest_context,
                    background_shell_backpressure_details(resolved_cwd, state, oldest),
                ),
            },
        )?;
        return Ok(());
    }

    let request_id = request.id.clone();
    let params = request.params.clone();
    let summary = summarize_tool_request(&params);
    let tool_name = tool.to_string();
    let worker_name = background_shell_worker_thread_name(tool, &request_id);
    let (target_background_shell_reference, target_background_shell_job_id) =
        async_background_shell_target(&params, &state.orchestration.background_shells);
    state.record_async_tool_request_with_timeout_and_worker(
        request_id.clone(),
        tool_name.clone(),
        summary.clone(),
        async_background_shell_timeout(tool, &params),
        worker_name.clone(),
    );
    if let Some(activity) = state.active_async_tool_requests.get_mut(&request_id) {
        activity.source_call_id = params
            .get("callId")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        activity.target_background_shell_reference = target_background_shell_reference;
        activity.target_background_shell_job_id = target_background_shell_job_id;
    }
    let resolved_cwd = resolved_cwd.to_string();
    let tx = tx.clone();
    let background_shells = state.orchestration.background_shells.clone();
    // Background-shell dynamic tools run on dedicated worker threads so a blocking
    // wrapper-side shell call cannot stall the main runtime loop forever.
    let spawn_result = thread::Builder::new().name(worker_name).spawn(move || {
        let result = execute_background_shell_tool_call_with_manager(
            &params,
            &resolved_cwd,
            &background_shells,
        );
        let _ = tx.send(AppEvent::AsyncToolResponseReady(AsyncToolResponse {
            id: request_id,
            tool: tool_name,
            summary,
            result,
        }));
    });
    if let Err(err) = spawn_result {
        state.finish_async_tool_request(&request.id);
        output.line_stderr(format!(
            "[self-supervision] failed to start dedicated async tool worker: {err}"
        ))?;
        send_json(
            writer,
            &OutgoingResponse {
                id: request.id.clone(),
                result: serde_json::json!({
                    "contentItems": [{
                        "type": "inputText",
                        "text": format!(
                            "dynamic tool `{tool}` could not start its dedicated wrapper worker thread; the request was failed locally instead of running on the main loop"
                        )
                    }],
                    "success": false
                }),
            },
        )?;
    }
    Ok(())
}

fn async_background_shell_timeout(tool: &str, params: &serde_json::Value) -> Duration {
    let arguments = params.get("arguments").unwrap_or(&serde_json::Value::Null);
    match tool {
        "background_shell_start" => Duration::from_millis(BACKGROUND_SHELL_START_TIMEOUT_MS),
        "background_shell_wait_ready" => {
            duration_from_optional_timeout(arguments.get("timeoutMs"), 60_000)
        }
        "background_shell_invoke_recipe" => {
            duration_from_optional_timeout(arguments.get("waitForReadyMs"), 60_000)
        }
        _ => Duration::from_millis(DEFAULT_BACKGROUND_SHELL_TOOL_TIMEOUT_MS),
    }
}

fn background_shell_worker_thread_name(tool: &str, request_id: &crate::rpc::RequestId) -> String {
    let request_suffix = match request_id {
        crate::rpc::RequestId::Integer(value) => value.to_string(),
        crate::rpc::RequestId::String(value) => value
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect(),
    };
    format!("codexw-bgtool-{tool}-{request_suffix}")
}

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
    request: Option<(
        &crate::rpc::RequestId,
        &crate::state::AbandonedAsyncToolRequest,
    )>,
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
    job: &crate::state::AsyncToolObservedBackgroundShellJob,
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

pub(super) fn summarize_abandoned_backpressure_context(
    state: &AppState,
    request: &crate::state::AbandonedAsyncToolRequest,
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

fn async_background_shell_target(
    params: &serde_json::Value,
    background_shells: &BackgroundShellManager,
) -> (Option<String>, Option<String>) {
    let requested_reference = params
        .get("arguments")
        .and_then(serde_json::Value::as_object)
        .and_then(|arguments| arguments.get("jobId"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|reference| !reference.is_empty())
        .map(ToOwned::to_owned);
    let resolved_job_id = requested_reference
        .as_deref()
        .and_then(|reference| background_shells.resolve_job_reference(reference).ok());
    (requested_reference, resolved_job_id)
}

fn duration_from_optional_timeout(value: Option<&serde_json::Value>, default_ms: u64) -> Duration {
    let requested_ms = value
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(default_ms);
    let bounded_ms = requested_ms
        .saturating_add(BACKGROUND_SHELL_REQUEST_TIMEOUT_GRACE_MS)
        .min(MAX_BACKGROUND_SHELL_TOOL_TIMEOUT_MS);
    Duration::from_millis(bounded_ms)
}
