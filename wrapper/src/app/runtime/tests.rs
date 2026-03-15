use crate::output::Output;
use crate::rpc::RequestId;
use crate::runtime_event_sources::AsyncToolResponse;
use crate::state::AppState;
use crate::state::AsyncToolHealthCheck;

use super::supervision::format_async_tool_health_check_line;
use super::supervision::format_supervision_notice_line;
use super::supervision::handle_async_tool_response;
use super::supervision::handle_supervision_tick;

use serde_json::json;
use std::process::Command;
use std::process::Stdio;

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

#[test]
fn async_tool_response_clears_active_request_tracking() {
    let mut state = AppState::new(true, false);
    state.record_async_tool_request(
        RequestId::Integer(42),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    handle_async_tool_response(
        AsyncToolResponse {
            id: RequestId::Integer(42),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            result: json!({"success": true}),
        },
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("handle async tool response");

    assert!(state.active_async_tool_requests.is_empty());
    assert!(state.active_supervision_notice.is_none());
}

#[test]
fn supervision_tick_tracks_raise_escalation_and_clear() {
    let mut state = AppState::new(true, false);
    state.record_async_tool_request(
        RequestId::Integer(7),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&RequestId::Integer(7))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(20);
    }
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    handle_supervision_tick(&mut state, &mut output, &mut writer).expect("raise slow notice");
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.classification.label()),
        Some("tool_slow")
    );

    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&RequestId::Integer(7))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(75);
    }
    handle_supervision_tick(&mut state, &mut output, &mut writer).expect("raise wedged notice");
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.classification.label()),
        Some("tool_wedged")
    );

    state.finish_async_tool_request(&RequestId::Integer(7));
    handle_supervision_tick(&mut state, &mut output, &mut writer).expect("clear notice");
    assert!(state.active_supervision_notice.is_none());
}

#[test]
fn format_supervision_notice_line_reports_owner_target_and_observation_details() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-7".to_string());
    state.turn_running = true;
    let line = format_supervision_notice_line(
        &crate::state::SupervisionNotice {
            classification: crate::state::AsyncToolSupervisionClass::ToolSlow,
            request_id: "7".to_string(),
            worker_thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-7".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-7".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state:
                crate::state::AsyncToolObservationState::WrapperBackgroundShellStreamingOutput,
            output_state: crate::state::AsyncToolOutputState::RecentOutputObserved,
            observed_background_shell_job: Some(
                crate::state::AsyncToolObservedBackgroundShellJob {
                    job_id: "bg-7".to_string(),
                    status: "running".to_string(),
                    command: "sleep 5".to_string(),
                    total_lines: 1,
                    last_output_age: Some(std::time::Duration::from_secs(2)),
                    recent_lines: vec!["READY".to_string()],
                },
            ),
        },
        &state,
        "/tmp/repo",
    );

    assert!(line.contains("[self-supervision] tool_slow background_shell_start"));
    assert!(line.contains("request=7"));
    assert!(line.contains("worker=codexw-bgtool-background_shell_start-7"));
    assert!(line.contains("owner=wrapper_background_shell"));
    assert!(line.contains("call=call-7"));
    assert!(line.contains("target=dev.api"));
    assert!(line.contains("resolved=bg-7"));
    assert!(line.contains("wrapper_background_shell_streaming_output|recent_output_observed"));
    assert!(line.contains("job=bg-7 running"));
    assert!(line.contains("[warn_only|observe_or_interrupt|automation_ready=false]"));
    assert!(line.contains("options=:status,:interrupt"));
}

#[test]
fn format_async_tool_health_check_line_reports_started_silent_job_details() {
    let line = format_async_tool_health_check_line(&AsyncToolHealthCheck {
        request_id: "9".to_string(),
        tool: "background_shell_start".to_string(),
        summary: "arguments= command=sleep 20 tool=background_shell_start".to_string(),
        owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
        source_call_id: Some("call-999".to_string()),
        target_background_shell_reference: Some("dev.api".to_string()),
        target_background_shell_job_id: Some("bg-9".to_string()),
        worker_thread_name: "codexw-bgtool-background_shell_start-9".to_string(),
        elapsed: std::time::Duration::from_secs(18),
        next_health_check_in: std::time::Duration::from_secs(5),
        supervision_classification: Some(crate::state::AsyncToolSupervisionClass::ToolSlow),
        observation_state:
            crate::state::AsyncToolObservationState::WrapperBackgroundShellStartedNoOutputYet,
        output_state: crate::state::AsyncToolOutputState::NoOutputObservedYet,
        observed_background_shell_job: Some(crate::state::AsyncToolObservedBackgroundShellJob {
            job_id: "bg-9".to_string(),
            status: "running".to_string(),
            command: "sleep 20".to_string(),
            total_lines: 0,
            last_output_age: None,
            recent_lines: Vec::new(),
        }),
    });

    assert!(line.contains("async worker check 18s"));
    assert!(line.contains("[tool_slow|observe_or_interrupt]"));
    assert!(line.contains("wrapper_background_shell_started_no_output_yet|no_output_observed_yet"));
    assert!(line.contains("call=call-999"));
    assert!(line.contains("target=dev.api"));
    assert!(line.contains("resolved=bg-9"));
    assert!(line.contains("job=bg-9 running"));
    assert!(line.contains("lines=0"));
    assert!(line.contains("command=sleep 20"));
    assert!(line.contains("next=5s"));
}

#[test]
fn format_async_tool_health_check_line_reports_streaming_output_details() {
    let line = format_async_tool_health_check_line(&AsyncToolHealthCheck {
        request_id: "10".to_string(),
        tool: "background_shell_start".to_string(),
        summary: "arguments= command=python stage2.py --quick tool=background_shell_start"
            .to_string(),
        owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
        source_call_id: Some("call-1000".to_string()),
        target_background_shell_reference: Some("dev.api".to_string()),
        target_background_shell_job_id: Some("bg-10".to_string()),
        worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
        elapsed: std::time::Duration::from_secs(24),
        next_health_check_in: std::time::Duration::from_secs(9),
        supervision_classification: Some(crate::state::AsyncToolSupervisionClass::ToolSlow),
        observation_state:
            crate::state::AsyncToolObservationState::WrapperBackgroundShellStreamingOutput,
        output_state: crate::state::AsyncToolOutputState::RecentOutputObserved,
        observed_background_shell_job: Some(crate::state::AsyncToolObservedBackgroundShellJob {
            job_id: "bg-10".to_string(),
            status: "running".to_string(),
            command: "python stage2.py --quick".to_string(),
            total_lines: 3,
            last_output_age: Some(std::time::Duration::from_secs(2)),
            recent_lines: vec!["stage1 ok".to_string(), "READY".to_string()],
        }),
    });

    assert!(line.contains("wrapper_background_shell_streaming_output|recent_output_observed"));
    assert!(line.contains("output_age=2s"));
    assert!(line.contains("output=READY"));
    assert!(line.contains("next=9s"));
    assert!(line.contains("target=dev.api"));
    assert!(line.contains("resolved=bg-10"));
    assert!(line.contains("job=bg-10 running"));
}

#[test]
fn supervision_tick_force_fails_timed_out_async_tool_requests() {
    let mut state = AppState::new(true, false);
    state.record_async_tool_request_with_timeout(
        RequestId::Integer(9),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        std::time::Duration::from_secs(1),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&RequestId::Integer(9))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(75);
    }
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    handle_supervision_tick(&mut state, &mut output, &mut writer)
        .expect("expire timed out async tool");

    assert!(state.active_async_tool_requests.is_empty());
    assert_eq!(state.abandoned_async_tool_request_count(), 1);
    assert!(state.active_supervision_notice.is_none());
}

#[test]
fn late_async_tool_response_clears_abandoned_request_after_timeout_cleanup() {
    let mut state = AppState::new(true, false);
    state.record_async_tool_request_with_timeout(
        RequestId::Integer(404),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        std::time::Duration::from_secs(1),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&RequestId::Integer(404))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(75);
    }
    let _expired = state.expire_timed_out_async_tool_requests();
    assert_eq!(state.abandoned_async_tool_request_count(), 1);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    handle_async_tool_response(
        AsyncToolResponse {
            id: RequestId::Integer(404),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            result: json!({"success": true}),
        },
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("drop late async tool response");

    assert!(state.active_async_tool_requests.is_empty());
    assert_eq!(state.abandoned_async_tool_request_count(), 0);
    assert!(state.active_supervision_notice.is_none());
}
