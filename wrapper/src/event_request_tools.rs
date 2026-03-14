use anyhow::Result;
use std::process::ChildStdin;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::background_shells::BackgroundShellManager;
use crate::client_dynamic_tools::execute_background_shell_tool_call_with_manager;
use crate::client_dynamic_tools::execute_dynamic_tool_call_with_state;
use crate::client_dynamic_tools::is_background_shell_tool;
use crate::client_dynamic_tools::legacy_workspace_tool_failure_notice;
use crate::client_dynamic_tools::legacy_workspace_tool_notice;
use crate::output::Output;
use crate::requests::send_json;
use crate::rpc::OutgoingResponse;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_event_sources::AsyncToolResponse;
use crate::state::AppState;
use crate::transcript_approval_summary::summarize_tool_request;
use crate::transcript_plan_render::build_mcp_elicitation_response;
use crate::transcript_plan_render::build_tool_user_input_response;

const DEFAULT_BACKGROUND_SHELL_TOOL_TIMEOUT_MS: u64 = 30_000;
const BACKGROUND_SHELL_START_TIMEOUT_MS: u64 = 15_000;
const BACKGROUND_SHELL_REQUEST_TIMEOUT_GRACE_MS: u64 = 5_000;
const MAX_BACKGROUND_SHELL_TOOL_TIMEOUT_MS: u64 = 300_000;

pub(crate) fn handle_tool_request(
    request: &RpcRequest,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<bool> {
    match request.method.as_str() {
        "tool/requestUserInput" | "item/tool/requestUserInput" => {
            let result = build_tool_user_input_response(&request.params);
            output.line_stderr(format!(
                "[input-request] auto-answered: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id.clone(),
                    result,
                },
            )?;
            Ok(true)
        }
        "mcpServer/elicitation/request" => {
            let result = build_mcp_elicitation_response(&request.params);
            let action = result
                .get("action")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("cancel");
            output.line_stderr(format!(
                "[input-request] auto-{action} MCP elicitation: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id.clone(),
                    result,
                },
            )?;
            Ok(true)
        }
        "item/tool/call" => {
            let tool = request
                .params
                .get("tool")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("dynamic tool");
            if is_background_shell_tool(tool) {
                if state.async_tool_backpressure_active() {
                    let backlog = state.abandoned_async_tool_request_count();
                    let oldest = state.oldest_abandoned_async_tool_request();
                    let oldest_summary = oldest
                        .map(|request| request.summary.as_str())
                        .unwrap_or("background-shell async backlog saturated");
                    let oldest_context = oldest
                        .map(|request| summarize_abandoned_backpressure_context(state, request))
                        .unwrap_or_default();
                    output.line_stderr(format!(
                        "[self-supervision] refusing async tool while abandoned backlog is saturated ({backlog}): {}{}",
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
                            ),
                        },
                    )?;
                    return Ok(true);
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
                return Ok(true);
            }
            let result = execute_dynamic_tool_call_with_state(&request.params, resolved_cwd, state);
            let success = result
                .get("success")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if let Some(notice) = legacy_workspace_tool_notice(tool) {
                output.line_stderr(notice)?;
            }
            if let Some(notice) = legacy_workspace_tool_failure_notice(tool, &result) {
                output.line_stderr(notice)?;
            }
            output.line_stderr(format!(
                "[tool] dynamic tool {}: {}",
                if success { "completed" } else { "failed" },
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id.clone(),
                    result,
                },
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
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

fn background_shell_backpressure_failure(
    tool: &str,
    backlog: usize,
    oldest_summary: &str,
    oldest_context: &str,
) -> serde_json::Value {
    serde_json::json!({
        "contentItems": [{
            "type": "inputText",
            "text": format!(
                "dynamic tool `{tool}` was refused locally because {backlog} timed-out background-shell worker(s) are still unresolved; newest work is blocked until the abandoned backlog drains or the operator interrupts/exits and resumes the thread; oldest backlog summary: {oldest_summary}{oldest_context}"
            )
        }],
        "success": false
    })
}

fn summarize_abandoned_backpressure_context(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::RequestId;
    use crate::runtime_event_sources::AsyncToolResponse;
    use serde_json::json;
    use std::process::Command;
    use std::process::Stdio;
    use std::time::Duration;

    fn spawn_sink_stdin() -> std::process::ChildStdin {
        Command::new("sh")
            .arg("-c")
            .arg("cat >/dev/null")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sink")
            .stdin
            .take()
            .expect("stdin")
    }

    fn test_request(method: &str, tool: &str, arguments: serde_json::Value) -> RpcRequest {
        RpcRequest {
            id: RequestId::Integer(7),
            method: method.to_string(),
            params: json!({
                "tool": tool,
                "threadId": "thread-1",
                "callId": "call-1",
                "arguments": arguments,
            }),
        }
    }

    #[test]
    fn background_shell_tool_requests_complete_asynchronously() {
        let mut state = AppState::new(true, false);
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();
        let (tx, rx) = mpsc::channel();
        let request = test_request(
            "item/tool/call",
            "background_shell_start",
            json!({"command": "printf 'alpha\\n'"}),
        );

        let handled =
            handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
                .expect("handle tool request");

        assert!(handled);
        assert_eq!(state.active_async_tool_requests.len(), 1);
        assert_eq!(
            state
                .active_async_tool_requests
                .get(&RequestId::Integer(7))
                .map(|activity| activity.worker_thread_name.as_str()),
            Some("codexw-bgtool-background_shell_start-7")
        );
        let event = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("async tool response");
        let AsyncToolResponse {
            id,
            tool,
            summary,
            result,
        } = match event {
            AppEvent::AsyncToolResponseReady(event) => event,
            other => panic!("expected async tool response, got {other:?}"),
        };
        assert_eq!(id, RequestId::Integer(7));
        assert_eq!(tool, "background_shell_start");
        assert!(summary.contains("background_shell_start"));
        assert_eq!(result["success"], true);
    }

    #[test]
    fn non_shell_dynamic_tool_requests_do_not_enqueue_async_response() {
        let mut state = AppState::new(true, false);
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();
        let (tx, rx) = mpsc::channel();
        let request = test_request("item/tool/call", "orchestration_status", json!({}));

        let handled =
            handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
                .expect("handle tool request");

        assert!(handled);
        assert!(state.active_async_tool_requests.is_empty());
        assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());
    }

    #[test]
    fn background_shell_start_uses_shorter_hard_timeout_than_default() {
        let mut state = AppState::new(true, false);
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();
        let (tx, _rx) = mpsc::channel();
        let request = test_request(
            "item/tool/call",
            "background_shell_start",
            json!({"command": "printf 'alpha\\n'"}),
        );

        let handled =
            handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
                .expect("handle tool request");

        assert!(handled);
        let activity = state
            .active_async_tool_requests
            .get(&RequestId::Integer(7))
            .expect("tracked async tool activity");
        assert_eq!(activity.hard_timeout, Duration::from_millis(15_000));
    }

    #[test]
    fn async_background_shell_request_tracks_resolved_target_job() {
        let mut state = AppState::new(true, false);
        let _ = state
            .orchestration
            .background_shells
            .start_from_tool_with_context(
                &json!({
                    "command": "echo READY; sleep 20",
                    "intent": "service",
                    "readyPattern": "READY",
                }),
                "/tmp",
                crate::background_shells::BackgroundShellOrigin::default(),
            );
        state
            .orchestration
            .background_shells
            .set_job_alias("bg-1", "dev.api")
            .expect("set alias");
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();
        let (tx, _rx) = mpsc::channel();
        let request = test_request(
            "item/tool/call",
            "background_shell_wait_ready",
            json!({"jobId": "dev.api", "timeoutMs": 60000}),
        );

        let handled =
            handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
                .expect("handle tool request");

        assert!(handled);
        let activity = state
            .active_async_tool_requests
            .get(&RequestId::Integer(7))
            .expect("tracked async tool activity");
        assert_eq!(
            activity.target_background_shell_reference.as_deref(),
            Some("dev.api")
        );
        assert_eq!(
            activity.target_background_shell_job_id.as_deref(),
            Some("bg-1")
        );
    }

    #[test]
    fn saturated_abandoned_async_backlog_refuses_new_shell_tool_requests() {
        let mut state = AppState::new(true, false);
        let mut output = Output::default();
        let mut writer = spawn_sink_stdin();
        let (tx, rx) = mpsc::channel();
        for id in 1..=crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS {
            state.record_async_tool_request_with_timeout(
                RequestId::Integer(id as i64),
                "background_shell_start".to_string(),
                format!("summary-{id}"),
                Duration::from_secs(1),
            );
            if let Some(activity) = state
                .active_async_tool_requests
                .get_mut(&RequestId::Integer(id as i64))
            {
                activity.started_at = std::time::Instant::now() - Duration::from_secs(10);
            }
        }
        let expired = state.expire_timed_out_async_tool_requests();
        assert_eq!(
            expired.len(),
            crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
        );
        let request = test_request(
            "item/tool/call",
            "background_shell_start",
            json!({"command": "printf 'alpha\\n'"}),
        );

        let handled =
            handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
                .expect("handle tool request");

        assert!(handled);
        assert!(state.active_async_tool_requests.is_empty());
        assert_eq!(
            state.abandoned_async_tool_request_count(),
            crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
        );
        assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());
    }

    #[test]
    fn abandoned_backpressure_context_includes_correlated_shell_observation() {
        let mut state = AppState::new(true, false);
        let _ = state
            .orchestration
            .background_shells
            .start_from_tool_with_context(
                &json!({
                    "command": "echo READY; sleep 20",
                    "intent": "service",
                    "readyPattern": "READY",
                }),
                "/tmp",
                crate::background_shells::BackgroundShellOrigin {
                    source_thread_id: Some("thread-1".to_string()),
                    source_call_id: Some("call-9".to_string()),
                    source_tool: Some("background_shell_wait_ready".to_string()),
                },
            );
        state
            .orchestration
            .background_shells
            .set_job_alias("bg-1", "dev.api")
            .expect("set alias");
        if let Ok(job) = state.orchestration.background_shells.lookup_job("bg-1") {
            let mut job = job.lock().expect("background shell job");
            job.total_lines = 1;
            job.last_output_at = Some(std::time::Instant::now());
            job.lines
                .push_back(crate::background_shells::BackgroundShellOutputLine {
                    cursor: 1,
                    text: "READY".to_string(),
                });
        }
        state.record_async_tool_request_with_timeout(
            RequestId::Integer(9),
            "background_shell_wait_ready".to_string(),
            "arguments= jobId=dev.api timeoutMs=60000 tool=background_shell_wait_ready".to_string(),
            Duration::from_secs(1),
        );
        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&RequestId::Integer(9))
        {
            activity.source_call_id = Some("call-9".to_string());
            activity.target_background_shell_reference = Some("dev.api".to_string());
            activity.target_background_shell_job_id = Some("bg-1".to_string());
            activity.started_at = std::time::Instant::now() - Duration::from_secs(30);
        }
        let _expired = state.expire_timed_out_async_tool_requests();
        let oldest = state
            .oldest_abandoned_async_tool_request()
            .expect("oldest abandoned request");

        let context = summarize_abandoned_backpressure_context(&state, oldest);
        let failure = background_shell_backpressure_failure(
            "background_shell_start",
            1,
            &oldest.summary,
            &context,
        );
        let failure_text = failure["contentItems"][0]["text"]
            .as_str()
            .expect("failure text");

        assert!(context.contains("wrapper_background_shell_streaming_output"));
        assert!(context.contains("recent_output_observed"));
        assert!(context.contains("call=call-9"));
        assert!(context.contains("target=dev.api resolved=bg-1"));
        assert!(context.contains("job=bg-1 running"));
        assert!(context.contains("output=READY"));
        assert!(failure_text.contains("oldest backlog summary"));
        assert!(failure_text.contains("job=bg-1 running"));
    }
}
