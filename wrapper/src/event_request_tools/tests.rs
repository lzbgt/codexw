use super::*;
use crate::output::Output;
use crate::rpc::RequestId;
use crate::rpc::RpcRequest;
use crate::runtime_event_sources::AsyncToolResponse;
use crate::state::AppState;
use serde_json::json;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;

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

fn spawn_recording_stdin() -> (TempDir, Child, std::process::ChildStdin, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("requests.jsonl");
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("cat > \"$1\"")
        .arg("sh")
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn recorder");
    let stdin = child.stdin.take().expect("stdin");
    (temp, child, stdin, path)
}

fn read_recorded_requests(
    child: &mut Child,
    writer: std::process::ChildStdin,
    path: &std::path::Path,
) -> Vec<serde_json::Value> {
    drop(writer);
    child.wait().expect("wait recorder");
    let contents = std::fs::read_to_string(path).expect("read requests");
    contents
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("parse request"))
        .collect()
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

    let handled = handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
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
        crate::runtime_event_sources::AppEvent::AsyncToolResponseReady(event) => event,
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

    let handled = handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
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

    let handled = handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
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

    let handled = handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
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
    state.realtime_session_id = Some("sess_test".to_string());
    state.thread_id = Some("thread_123".to_string());
    state.turn_running = true;
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();
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
            activity.started_at =
                std::time::Instant::now() - Duration::from_secs(if id == 1 { 20 } else { 10 });
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

    let handled = handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
        .expect("handle tool request");

    assert!(handled);
    assert!(state.active_async_tool_requests.is_empty());
    assert_eq!(
        state.abandoned_async_tool_request_count(),
        crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    );
    assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());
    let responses = read_recorded_requests(&mut child, writer, &path);
    assert_eq!(responses.len(), 1);
    let result = &responses[0]["result"];
    assert_eq!(result["success"], false);
    assert_eq!(result["failure_kind"], "async_tool_backpressure");
    assert_eq!(
        result["backpressure"]["abandoned_request_count"],
        crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    );
    assert_eq!(result["backpressure"]["saturated"], true);
    assert_eq!(
        result["backpressure"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        result["backpressure"]["recovery_policy"]["kind"],
        "operator_interrupt_or_exit_resume"
    );
    assert_eq!(
        result["backpressure"]["recovery_policy"]["automation_ready"],
        false
    );
    assert_eq!(
        result["backpressure"]["recovery_options"][0]["kind"],
        "observe_status"
    );
    assert_eq!(
        result["backpressure"]["recovery_options"][1]["local_api_path"],
        "/api/v1/session/sess_test/turn/interrupt"
    );
    assert!(
        result["backpressure"]["recovery_options"][2]["cli_command"]
            .as_str()
            .expect("resume command")
            .ends_with(" --cwd /tmp resume thread_123")
    );
    assert_eq!(
        result["backpressure"]["oldest_tool"],
        "background_shell_start"
    );
    let oldest_summary = result["backpressure"]["oldest_summary"]
        .as_str()
        .expect("oldest summary");
    assert_eq!(oldest_summary, "summary-1");
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
        .oldest_abandoned_async_tool_entry()
        .expect("oldest abandoned request");

    let context =
        super::background_shells::summarize_abandoned_backpressure_context(&state, oldest.1);
    let failure = super::background_shells::background_shell_backpressure_failure(
        "background_shell_start",
        1,
        &oldest.1.summary,
        &context,
        super::background_shells::background_shell_backpressure_details(
            "/tmp",
            &state,
            Some(oldest),
        ),
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
    assert_eq!(failure["failure_kind"], "async_tool_backpressure");
    assert_eq!(
        failure["backpressure"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        failure["backpressure"]["recovery_policy"]["kind"],
        "warn_only"
    );
    assert_eq!(failure["backpressure"]["oldest_request_id"], "9");
    assert_eq!(
        failure["backpressure"]["oldest_thread_name"],
        "codexw-async-tool-worker-9"
    );
    assert_eq!(failure["backpressure"]["oldest_source_call_id"], "call-9");
    assert_eq!(
        failure["backpressure"]["oldest_target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        failure["backpressure"]["oldest_target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        failure["backpressure"]["oldest_observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        failure["backpressure"]["oldest_output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        failure["backpressure"]["oldest_observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        failure["backpressure"]["oldest_observed_background_shell_job"]["recent_lines"][0],
        "READY"
    );
}
