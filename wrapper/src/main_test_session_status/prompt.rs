use super::*;
use serde_json::json;
use std::time::Duration;
use std::time::Instant;

#[test]
fn thread_status_summary_prefers_human_flags() {
    assert_eq!(
        summarize_thread_status_for_display(&json!({
            "status": {"type": "active", "activeFlags": ["waitingOnApproval"]}
        })),
        Some("waiting on approval".to_string())
    );
    assert_eq!(
        summarize_thread_status_for_display(&json!({
            "status": {"type": "idle", "activeFlags": []}
        })),
        Some("ready".to_string())
    );
}

#[test]
fn prompt_status_uses_active_detail_when_present() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.started_turn_count = 2;
    state.last_status_line = Some("waiting on approval".to_string());
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("waiting on approval"));
}

#[test]
fn active_spinner_uses_codex_braille_frames() {
    assert_eq!(spinner_frame(None), "⠋");
    let now = Instant::now();
    assert_eq!(spinner_frame(Some(now - Duration::from_millis(100))), "⠙");
    assert_eq!(spinner_frame(Some(now - Duration::from_millis(700))), "⠇");
}

#[test]
fn prompt_status_mentions_realtime_when_active() {
    let mut state = crate::state::AppState::new(true, false);
    state.realtime_active = true;
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("realtime"));
}

#[test]
fn prompt_status_mentions_async_tool_activity_when_present() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.record_async_tool_request(
        crate::rpc::RequestId::Integer(7),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("async tool background_shell_start"));
    assert!(rendered.contains("background_shell_start"));
    assert!(rendered.contains("awaiting shell start/output"));
    assert!(rendered.contains("no output yet"));
    assert!(rendered.contains("next check"));
}

#[test]
fn prompt_status_mentions_async_tool_supervision_class_when_slow() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(8),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-8".to_string(),
            started_at: Instant::now() - Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("tool_slow"));
    assert!(rendered.contains("async tool background_shell_start"));
    assert!(rendered.contains("observe or interrupt"));
}

#[test]
fn prompt_status_mentions_correlated_background_job_output_when_observed() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(18),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=printf READY tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-18".to_string()),
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-18".to_string(),
            started_at: Instant::now() - Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    let _ = state
        .orchestration
        .background_shells
        .start_from_tool_with_context(
            &json!({
                "command": "echo READY; sleep 2",
                "intent": "observation",
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-1".to_string()),
                source_call_id: Some("call-18".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        );
    if let Ok(job) = state.orchestration.background_shells.lookup_job("bg-1") {
        let mut job = job.lock().expect("background shell job");
        job.total_lines = 1;
        job.last_output_at = Some(Instant::now());
        job.lines
            .push_back(crate::background_shells::BackgroundShellOutputLine {
                cursor: 1,
                text: "READY".to_string(),
            });
    }
    let observation = state.async_tool_observation(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(18))
            .expect("active async tool"),
    );

    let rendered = render_prompt_status(&state);

    assert_eq!(observation.output_state.label(), "recent_output_observed");
    assert!(rendered.contains("wrapper bg shell"));
    assert!(rendered.contains("job bg-1"));
    assert!(rendered.contains("recent output"));
    assert!(rendered.contains("READY"));
    assert!(observation.observed_background_shell_job.is_some());
}

#[test]
fn prompt_status_mentions_correlated_background_job_started_without_output_yet() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(19),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 20 tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-19".to_string()),
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-19".to_string(),
            started_at: Instant::now() - Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    let _ = state
        .orchestration
        .background_shells
        .start_from_tool_with_context(
            &json!({
                "command": "sleep 20",
                "intent": "observation",
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-1".to_string()),
                source_call_id: Some("call-19".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        );

    let observation = state.async_tool_observation(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(19))
            .expect("active async tool"),
    );

    let rendered = render_prompt_status(&state);

    assert_eq!(
        observation.observation_state.label(),
        "wrapper_background_shell_started_no_output_yet"
    );
    assert_eq!(observation.output_state.label(), "no_output_observed_yet");
    assert!(rendered.contains("job started; awaiting output"));
    assert!(rendered.contains("no output yet"));
    assert!(rendered.contains("job bg-1 running"));
    assert!(observation.observed_background_shell_job.is_some());
}

#[test]
fn prompt_status_correlates_wait_ready_to_target_background_job() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
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
    if let Ok(job) = state.orchestration.background_shells.lookup_job("bg-1") {
        let mut job = job.lock().expect("background shell job");
        job.total_lines = 1;
        job.last_output_at = Some(Instant::now());
        job.lines
            .push_back(crate::background_shells::BackgroundShellOutputLine {
                cursor: 1,
                text: "READY".to_string(),
            });
    }
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(20),
        crate::state::AsyncToolActivity {
            tool: "background_shell_wait_ready".to_string(),
            summary: "arguments= jobId=dev.api timeoutMs=60000 tool=background_shell_wait_ready"
                .to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-1".to_string()),
            worker_thread_name: "codexw-bgtool-background_shell_wait_ready-20".to_string(),
            started_at: Instant::now() - Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );

    let observation = state.async_tool_observation(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(20))
            .expect("active async tool"),
    );
    let rendered = render_prompt_status(&state);

    assert_eq!(
        observation.observation_state.label(),
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(observation.output_state.label(), "recent_output_observed");
    assert!(rendered.contains("async tool background_shell_wait_ready"));
    assert!(rendered.contains("job bg-1"));
    assert!(rendered.contains("READY"));
    assert!(observation.observed_background_shell_job.is_some());
}

#[test]
fn prompt_status_mentions_abandoned_async_backlog_when_no_active_tool_remains() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.record_async_tool_request_with_timeout(
        crate::rpc::RequestId::Integer(11),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        Duration::from_secs(1),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(11))
    {
        activity.started_at = Instant::now() - Duration::from_secs(90);
    }
    let _expired = state.expire_timed_out_async_tool_requests();

    let rendered = render_prompt_status(&state);

    assert!(rendered.contains("async backlog 1"));
    assert!(rendered.contains("background_shell_start"));
}

#[test]
fn prompt_status_mentions_saturated_async_backlog() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    for id in 1..=crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS {
        state.record_async_tool_request_with_timeout(
            crate::rpc::RequestId::Integer(id as i64),
            "background_shell_start".to_string(),
            format!("summary-{id}"),
            Duration::from_secs(1),
        );
        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&crate::rpc::RequestId::Integer(id as i64))
        {
            activity.started_at = Instant::now() - Duration::from_secs(90);
        }
    }
    let _expired = state.expire_timed_out_async_tool_requests();

    let rendered = render_prompt_status(&state);

    assert!(rendered.contains("async backlog saturated"));
}

#[test]
fn prompt_status_mentions_startup_resume_picker() {
    let mut state = crate::state::AppState::new(true, false);
    state.startup_resume_picker = true;
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("resume picker"));
    assert!(rendered.contains(" | "));
    assert!(rendered.contains("/new"));
}

#[test]
fn prompt_status_ready_includes_collaboration_and_personality() {
    let mut state = crate::state::AppState::new(true, false);
    state.completed_turn_count = 3;
    state.active_personality = Some("pragmatic".to_string());
    state.active_collaboration_mode = Some(crate::collaboration_preset::CollaborationModePreset {
        name: "Plan".to_string(),
        mode_kind: Some("plan".to_string()),
        model: Some("gpt-5-codex".to_string()),
        reasoning_effort: Some(Some("high".to_string())),
    });
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("plan mode"));
    assert!(rendered.contains("Pragmatic"));
    assert!(rendered.contains("3 turns"));
    assert!(rendered.contains(" | "));
}
