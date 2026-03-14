use super::*;

#[test]
fn status_snapshot_includes_realtime_fields() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.realtime_active = true;
    state.realtime_session_id = Some("rt-1".to_string());
    state.realtime_prompt = Some("hello world".to_string());
    state.realtime_last_error = Some("bad gateway".to_string());
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });
    let mut lines = render_status_overview(&cli, "/tmp/project", &state);
    lines.extend(render_status_runtime(&cli, &state));
    let rendered = lines.join("\n");
    assert!(rendered.contains("realtime        true"));
    assert!(rendered.contains("realtime id     rt-1"));
    assert!(rendered.contains("realtime prompt hello world"));
    assert!(rendered.contains("realtime error  bad gateway"));
}

#[test]
fn status_snapshot_includes_async_tool_fields() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.record_async_tool_request(
        crate::rpc::RequestId::Integer(3),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });
    let rendered = render_status_runtime(&cli, &state).join("\n");
    assert!(rendered.contains("async tools     1"));
    assert!(
        rendered.contains("async tool      arguments= command=sleep 5 tool=background_shell_start")
    );
    assert!(rendered.contains("async req       3"));
    assert!(
        rendered.contains("async thread    codexw-async-tool-worker-3")
            || rendered.contains("async thread    codexw-bgtool-background_shell_start-3")
    );
    assert!(rendered.contains("async owner     wrapper_background_shell"));
    assert!(rendered.contains("async obs       no_job_or_output_observed_yet"));
    assert!(rendered.contains("async out       no_output_observed_yet"));
    assert!(rendered.contains("async chk in"));
    assert!(rendered.contains("async time"));
    assert!(rendered.contains("async worker    running"));
    assert!(rendered.contains("async worker id 3"));
    assert!(rendered.contains("async worker ow wrapper_background_shell"));
    assert!(rendered.contains("async worker ob no_job_or_output_observed_yet"));
    assert!(rendered.contains("async worker os no_output_observed_yet"));
    assert!(rendered.contains("async worker ck"));
}

#[test]
fn status_snapshot_includes_async_tool_supervision_classification() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(13),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-13".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(65),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });
    let rendered = render_status_runtime(&cli, &state).join("\n");
    assert!(rendered.contains("async class     tool_wedged"));
    assert!(rendered.contains("async action    interrupt_or_exit_resume"));
    assert!(rendered.contains("async req       13"));
    assert!(rendered.contains("async thread    codexw-bgtool-background_shell_start-13"));
    assert!(rendered.contains("supervision     tool_wedged background_shell_start"));
    assert!(rendered.contains("supervision pol operator_interrupt_or_exit_resume"));
    assert!(rendered.contains("supervision act interrupt_or_exit_resume"));
}

#[test]
fn oldest_active_async_tool_entry_is_deterministic_on_started_at_ties() {
    let mut state = crate::state::AppState::new(true, false);
    let started_at = std::time::Instant::now() - std::time::Duration::from_secs(20);
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(9),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "summary-9".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_wait_ready-9".to_string(),
            started_at,
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(8),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "summary-8".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-8".to_string(),
            started_at,
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );

    let (request_id, activity) = state
        .oldest_async_tool_entry()
        .expect("deterministic oldest active async tool");

    assert_eq!(crate::state::request_id_label(request_id), "8");
    assert_eq!(activity.summary, "summary-8");
}

#[test]
fn status_snapshot_includes_correlated_background_shell_job_details() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(31),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=printf READY tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-31".to_string()),
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-31".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(20),
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
            &serde_json::json!({
                "command": "echo READY; sleep 2",
                "intent": "observation",
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-1".to_string()),
                source_call_id: Some("call-31".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        );
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
    let observation = state.async_tool_observation(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(31))
            .expect("active async tool"),
    );
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });

    let rendered = render_status_runtime(&cli, &state).join("\n");

    assert_eq!(observation.output_state.label(), "recent_output_observed");
    assert!(rendered.contains("async call      call-31"));
    assert!(rendered.contains("async job       bg-1 "));
    assert!(rendered.contains("async cmd       echo READY; sleep 2"));
    assert!(rendered.contains("async out       recent_output_observed"));
    assert!(rendered.contains("async out age"));
    assert!(rendered.contains("async worker cl call-31"));
    assert!(rendered.contains("async worker jb bg-1 "));
    assert!(rendered.contains("async worker os recent_output_observed"));
    assert!(rendered.contains("async worker oa"));
    assert!(observation.observed_background_shell_job.is_some());
}

#[test]
fn status_snapshot_marks_correlated_background_shell_output_as_stale() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(41),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 2 tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-41".to_string()),
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-41".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(70),
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
            &serde_json::json!({
                "command": "sleep 2",
                "intent": "observation",
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-1".to_string()),
                source_call_id: Some("call-41".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        );
    if let Ok(job) = state.orchestration.background_shells.lookup_job("bg-1") {
        let mut job = job.lock().expect("background shell job");
        job.total_lines = 1;
        job.last_output_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(75));
        job.lines
            .push_back(crate::background_shells::BackgroundShellOutputLine {
                cursor: 1,
                text: "still waiting".to_string(),
            });
    }
    let observation = state.async_tool_observation(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(41))
            .expect("active async tool"),
    );
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });

    let rendered = render_status_runtime(&cli, &state).join("\n");

    assert_eq!(observation.output_state.label(), "stale_output_observed");
    assert!(rendered.contains("async out       stale_output_observed"));
    assert!(rendered.contains("async out age"));
    assert!(rendered.contains("async output    still waiting"));
    assert!(rendered.contains("async worker os stale_output_observed"));
    assert!(rendered.contains("async worker oa"));
}

#[test]
fn status_snapshot_correlates_wait_ready_to_target_background_job() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    let _ = state
        .orchestration
        .background_shells
        .start_from_tool_with_context(
            &serde_json::json!({
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
        job.last_output_at = Some(std::time::Instant::now());
        job.lines
            .push_back(crate::background_shells::BackgroundShellOutputLine {
                cursor: 1,
                text: "READY".to_string(),
            });
    }
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(43),
        crate::state::AsyncToolActivity {
            tool: "background_shell_wait_ready".to_string(),
            summary: "arguments= jobId=dev.api timeoutMs=60000 tool=background_shell_wait_ready"
                .to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-1".to_string()),
            worker_thread_name: "codexw-bgtool-background_shell_wait_ready-43".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    let observation = state.async_tool_observation(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(43))
            .expect("active async tool"),
    );
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });

    let rendered = render_status_runtime(&cli, &state).join("\n");

    assert_eq!(
        observation.observation_state.label(),
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(observation.output_state.label(), "recent_output_observed");
    assert!(rendered.contains(
        "async tool      arguments= jobId=dev.api timeoutMs=60000 tool=background_shell_wait_ready"
    ));
    assert!(rendered.contains("async target    dev.api"));
    assert!(rendered.contains("async target jb bg-1"));
    assert!(rendered.contains("async job       bg-1 "));
    assert!(rendered.contains("async cmd       echo READY; sleep 20"));
    assert!(rendered.contains("async out       recent_output_observed"));
    assert!(rendered.contains("async worker tr dev.api"));
    assert!(rendered.contains("async worker tj bg-1"));
    assert!(rendered.contains("async worker jb bg-1 "));
    assert!(rendered.contains("async worker os recent_output_observed"));
    assert!(observation.observed_background_shell_job.is_some());
}

#[test]
fn status_snapshot_marks_correlated_background_shell_started_without_output_yet() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(42),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 20 tool=background_shell_start".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: Some("call-42".to_string()),
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-42".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(25),
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
            &serde_json::json!({
                "command": "sleep 20",
                "intent": "observation",
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-1".to_string()),
                source_call_id: Some("call-42".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        );
    let observation = state.async_tool_observation(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(42))
            .expect("active async tool"),
    );
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });

    let rendered = render_status_runtime(&cli, &state).join("\n");

    assert_eq!(
        observation.observation_state.label(),
        "wrapper_background_shell_started_no_output_yet"
    );
    assert_eq!(observation.output_state.label(), "no_output_observed_yet");
    assert!(rendered.contains("async obs       wrapper_background_shell_started_no_output_yet"));
    assert!(rendered.contains("async out       no_output_observed_yet"));
    assert!(rendered.contains("async job       bg-1 running"));
    assert!(rendered.contains("async lines     0"));
    assert!(rendered.contains("async worker ob wrapper_background_shell_started_no_output_yet"));
    assert!(rendered.contains("async worker os no_output_observed_yet"));
    assert!(observation.observed_background_shell_job.is_some());
}

#[test]
fn status_snapshot_includes_abandoned_async_backpressure() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    let _ = state
        .orchestration
        .background_shells
        .start_from_tool_with_context(
            &serde_json::json!({
                "command": "echo READY; sleep 20",
                "intent": "service",
                "readyPattern": "READY",
            }),
            "/tmp",
            crate::background_shells::BackgroundShellOrigin {
                source_thread_id: Some("thread-1".to_string()),
                source_call_id: Some("call-21".to_string()),
                source_tool: Some("background_shell_start".to_string()),
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
        crate::rpc::RequestId::Integer(21),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        std::time::Duration::from_secs(1),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(21))
    {
        activity.source_call_id = Some("call-21".to_string());
        activity.target_background_shell_reference = Some("dev.api".to_string());
        activity.target_background_shell_job_id = Some("bg-1".to_string());
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(90);
    }
    let _expired = state.expire_timed_out_async_tool_requests();
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    });
    let rendered = render_status_runtime(&cli, &state).join("\n");
    assert!(rendered.contains("async aban      1"));
    assert!(rendered.contains("async stale rq  21"));
    assert!(rendered.contains("async stale wk  codexw-async-tool-worker-21"));
    assert!(
        rendered.contains("async stale     arguments= command=sleep 5 tool=background_shell_start")
    );
    assert!(rendered.contains("async stale cl  call-21"));
    assert!(rendered.contains("async stale tr  dev.api"));
    assert!(rendered.contains("async stale tj  bg-1"));
    assert!(rendered.contains("async stale ob  wrapper_background_shell_streaming_output"));
    assert!(rendered.contains("async stale os  recent_output_observed"));
    assert!(rendered.contains("async stale jb  bg-1 running"));
    assert!(rendered.contains("async stale ot  READY"));
    assert!(rendered.contains("async worker    abandoned_after_timeout"));
    assert!(rendered.contains("async worker id 21"));
    assert!(rendered.contains("async worker cl call-21"));
    assert!(rendered.contains("async worker tr dev.api"));
    assert!(rendered.contains("async worker tj bg-1"));
    assert!(rendered.contains("async worker ob wrapper_background_shell_streaming_output"));
    assert!(rendered.contains("async worker os recent_output_observed"));
    assert!(rendered.contains("async worker jb bg-1 running"));
    assert!(rendered.contains("async worker ot READY"));
    assert!(rendered.contains("async guard     monitoring"));
}

#[test]
fn resetting_thread_context_clears_stream_buffers() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .command_output_buffers
        .insert("cmd-1".to_string(), "out".to_string());
    state
        .file_output_buffers
        .insert("file-1".to_string(), "diff".to_string());
    state.process_output_buffers.insert(
        "proc-1".to_string(),
        crate::state::ProcessOutputBuffer {
            stdout: "stdout".to_string(),
            stderr: "stderr".to_string(),
        },
    );
    state.last_agent_message = Some("reply".to_string());
    state.last_turn_diff = Some("diff".to_string());
    state.last_status_line = Some("running".to_string());
    state.record_async_tool_request(
        crate::rpc::RequestId::Integer(4),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["thread-agent-1".to_string()],
            prompt: Some("inspect auth".to_string()),
            agent_statuses: std::collections::BTreeMap::from([(
                "thread-agent-1".to_string(),
                "running".to_string(),
            )]),
        },
    );

    state.reset_thread_context();

    assert!(state.command_output_buffers.is_empty());
    assert!(state.file_output_buffers.is_empty());
    assert!(state.process_output_buffers.is_empty());
    assert!(state.last_agent_message.is_none());
    assert!(state.last_turn_diff.is_none());
    assert!(state.last_status_line.is_none());
    assert!(state.active_async_tool_requests.is_empty());
    assert!(state.live_agent_tasks.is_empty());
    assert!(!state.startup_resume_picker);
}

#[test]
fn finishing_async_tool_request_removes_it_from_status_tracking() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request(
        crate::rpc::RequestId::Integer(9),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );

    let removed = state.finish_async_tool_request(&crate::rpc::RequestId::Integer(9));

    assert!(removed.is_some());
    assert!(state.active_async_tool_requests.is_empty());
    assert!(state.oldest_async_tool_activity().is_none());
}

#[test]
fn expiring_async_tool_requests_moves_them_into_abandoned_backlog() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout(
        crate::rpc::RequestId::Integer(15),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        std::time::Duration::from_secs(1),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(15))
    {
        activity.source_call_id = Some("call-15".to_string());
        activity.target_background_shell_reference = Some("dev.api".to_string());
        activity.target_background_shell_job_id = Some("bg-1".to_string());
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(80);
    }

    let expired = state.expire_timed_out_async_tool_requests();

    assert_eq!(expired.len(), 1);
    assert!(state.active_async_tool_requests.is_empty());
    assert_eq!(state.abandoned_async_tool_request_count(), 1);
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .map(|request| request.tool.as_str()),
        Some("background_shell_start")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .map(|request| request.worker_thread_name.as_str()),
        Some("codexw-async-tool-worker-15")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .and_then(|request| request.source_call_id.as_deref()),
        Some("call-15")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .and_then(|request| request.target_background_shell_reference.as_deref()),
        Some("dev.api")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .and_then(|request| request.target_background_shell_job_id.as_deref()),
        Some("bg-1")
    );
    assert!(!state.async_tool_backpressure_active());
}

#[test]
fn async_tool_worker_statuses_expose_running_and_abandoned_workers() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(7),
        "background_shell_start".to_string(),
        "running".to_string(),
        std::time::Duration::from_secs(30),
        "codexw-bgtool-background_shell_start-7".to_string(),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(7))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(20);
    }
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(8),
        "background_shell_start".to_string(),
        "abandoned".to_string(),
        std::time::Duration::from_secs(5),
        "codexw-bgtool-background_shell_start-8".to_string(),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(8))
    {
        activity.source_call_id = Some("call-8".to_string());
        activity.target_background_shell_reference = Some("dev.api".to_string());
        activity.target_background_shell_job_id = Some("bg-1".to_string());
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(80);
    }
    let _expired = state.expire_timed_out_async_tool_requests();

    let workers = state.async_tool_worker_statuses();

    assert_eq!(workers.len(), 2);
    assert_eq!(workers[0].request_id, "7");
    assert_eq!(workers[0].lifecycle_state.label(), "running");
    assert_eq!(
        workers[0].worker_thread_name,
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        workers[0]
            .supervision_classification
            .map(|classification| classification.label()),
        Some("tool_slow")
    );
    assert_eq!(
        workers[0]
            .observation_state
            .map(|observation_state| observation_state.label()),
        Some("no_job_or_output_observed_yet")
    );
    assert_eq!(
        workers[0]
            .output_state
            .map(|output_state| output_state.label()),
        Some("no_output_observed_yet")
    );
    assert_eq!(workers[0].owner_kind.label(), "wrapper_background_shell");
    assert!(workers[0].source_call_id.is_none());
    assert!(workers[0].next_health_check_in.is_some());
    assert_eq!(workers[1].request_id, "8");
    assert_eq!(
        workers[1].lifecycle_state.label(),
        "abandoned_after_timeout"
    );
    assert_eq!(
        workers[1].worker_thread_name,
        "codexw-bgtool-background_shell_start-8"
    );
    assert_eq!(workers[1].supervision_classification, None);
    assert_eq!(
        workers[1]
            .observation_state
            .map(|observation_state| observation_state.label()),
        Some("no_job_or_output_observed_yet")
    );
    assert_eq!(
        workers[1]
            .output_state
            .map(|output_state| output_state.label()),
        Some("no_output_observed_yet")
    );
    assert_eq!(workers[1].source_call_id.as_deref(), Some("call-8"));
    assert_eq!(
        workers[1].target_background_shell_reference.as_deref(),
        Some("dev.api")
    );
    assert_eq!(
        workers[1].target_background_shell_job_id.as_deref(),
        Some("bg-1")
    );
    assert!(workers[1].observed_background_shell_job.is_none());
    assert_eq!(workers[1].next_health_check_in, None);
}

#[test]
fn abandoned_async_backlog_becomes_saturated_at_threshold() {
    let mut state = crate::state::AppState::new(true, false);
    for id in 1..=crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS {
        state.record_async_tool_request_with_timeout(
            crate::rpc::RequestId::Integer(id as i64),
            "background_shell_start".to_string(),
            format!("summary-{id}"),
            std::time::Duration::from_secs(1),
        );
        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&crate::rpc::RequestId::Integer(id as i64))
        {
            activity.started_at = std::time::Instant::now()
                - std::time::Duration::from_secs(if id == 1 { 120 } else { 80 });
        }
    }

    let expired = state.expire_timed_out_async_tool_requests();

    assert_eq!(
        expired.len(),
        crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    );
    assert!(state.async_tool_backpressure_active());
    assert_eq!(
        state.abandoned_async_tool_request_count(),
        crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .map(|request| request.summary.as_str()),
        Some("summary-1")
    );
}

#[test]
fn async_tool_supervision_classifies_slow_and_wedged_elapsed_time() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "slow".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.label()),
        Some("tool_slow")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recommended_action()),
        Some("observe_or_interrupt")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recovery_policy_kind().label()),
        Some("warn_only")
    );

    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "wedged".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(65),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.label()),
        Some("tool_wedged")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recommended_action()),
        Some("interrupt_or_exit_resume")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recovery_policy_kind().label()),
        Some("operator_interrupt_or_exit_resume")
    );
}

#[test]
fn async_tool_supervision_notice_tracks_raise_escalation_and_clear() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "slow".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );

    let raised = state.refresh_async_tool_supervision_notice();
    assert!(matches!(
        raised,
        Some(crate::state::SupervisionNoticeTransition::Raised(_))
    ));
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.classification.label()),
        Some("tool_slow")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.recovery_policy_kind().label()),
        Some("warn_only")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.request_id.as_str()),
        Some("10")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.worker_thread_name.as_str()),
        Some("codexw-bgtool-background_shell_start-10")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.owner_kind.label()),
        Some("wrapper_background_shell")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.observation_state.label()),
        Some("no_job_or_output_observed_yet")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.output_state.label()),
        Some("no_output_observed_yet")
    );

    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "wedged".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(75),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    let escalated = state.refresh_async_tool_supervision_notice();
    assert!(matches!(
        escalated,
        Some(crate::state::SupervisionNoticeTransition::Raised(_))
    ));
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.classification.label()),
        Some("tool_wedged")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.recovery_policy_kind().label()),
        Some("operator_interrupt_or_exit_resume")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.request_id.as_str()),
        Some("10")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.worker_thread_name.as_str()),
        Some("codexw-bgtool-background_shell_start-10")
    );

    state.active_async_tool_requests.clear();
    let cleared = state.refresh_async_tool_supervision_notice();
    assert_eq!(
        cleared,
        Some(crate::state::SupervisionNoticeTransition::Cleared)
    );
    assert!(state.active_supervision_notice.is_none());
}

#[test]
fn async_tool_health_checks_are_scheduled_by_orchestrator_policy() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(77),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        std::time::Duration::from_secs(15),
        "codexw-bgtool-background_shell_start-77".to_string(),
    );
    let activity = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(77))
        .expect("activity");
    assert_eq!(activity.next_health_check_after.as_secs(), 5);

    activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(6);
    let checks = state.collect_due_async_tool_health_checks();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].request_id, "77");
    assert_eq!(checks[0].tool, "background_shell_start");
    assert!(checks[0].summary.contains("command=sleep 5"));
    assert_eq!(checks[0].supervision_classification, None);
    assert_eq!(
        checks[0].observation_state.label(),
        "no_job_or_output_observed_yet"
    );

    let activity = state
        .active_async_tool_requests
        .get(&crate::rpc::RequestId::Integer(77))
        .expect("activity");
    assert_eq!(activity.next_health_check_after.as_secs(), 11);
}

#[test]
fn async_tool_health_checks_escalate_when_worker_becomes_slow() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(78),
        "background_shell_wait_ready".to_string(),
        "jobId=bg-1 timeoutMs=60000".to_string(),
        std::time::Duration::from_secs(65),
        "codexw-bgtool-background_shell_wait_ready-78".to_string(),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(78))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(20);
        activity.next_health_check_after = std::time::Duration::from_secs(15);
    }

    let checks = state.collect_due_async_tool_health_checks();

    assert_eq!(checks.len(), 1);
    assert_eq!(
        checks[0]
            .supervision_classification
            .map(|classification| classification.label()),
        Some("tool_slow")
    );
    assert_eq!(
        checks[0].observation_state.label(),
        "no_job_or_output_observed_yet"
    );
    assert_eq!(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(78))
            .expect("activity")
            .next_health_check_after
            .as_secs(),
        35
    );
}
