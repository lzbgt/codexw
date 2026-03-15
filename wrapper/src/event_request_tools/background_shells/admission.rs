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
use crate::rpc::RequestId;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_event_sources::AsyncToolResponse;
use crate::state::AppState;
use crate::transcript_approval_summary::summarize_tool_request;

use super::BACKGROUND_SHELL_REQUEST_TIMEOUT_GRACE_MS;
use super::BACKGROUND_SHELL_START_TIMEOUT_MS;
use super::DEFAULT_BACKGROUND_SHELL_TOOL_TIMEOUT_MS;
use super::MAX_BACKGROUND_SHELL_TOOL_TIMEOUT_MS;
use super::backpressure::background_shell_backpressure_details;
use super::backpressure::background_shell_backpressure_failure;
use super::backpressure::summarize_abandoned_backpressure_context;

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

fn background_shell_worker_thread_name(tool: &str, request_id: &RequestId) -> String {
    let request_suffix = match request_id {
        RequestId::Integer(value) => value.to_string(),
        RequestId::String(value) => value
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect(),
    };
    format!("codexw-bgtool-{tool}-{request_suffix}")
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
