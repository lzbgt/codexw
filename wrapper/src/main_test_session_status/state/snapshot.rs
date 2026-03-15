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
    assert!(rendered.contains("supervision req 13"));
    assert!(rendered.contains("supervision th  codexw-bgtool-background_shell_start-13"));
    assert!(rendered.contains("supervision ow  wrapper_background_shell"));
    assert!(rendered.contains("supervision auto false"));
    assert!(
        rendered.contains("supervision sum arguments= command=sleep 5 tool=background_shell_start")
    );
    assert!(rendered.contains("supervision ob  no_job_or_output_observed_yet"));
    assert!(rendered.contains("supervision os  no_output_observed_yet"));
    assert!(rendered.contains("supervision opt interrupt_turn :interrupt"));
    assert!(rendered.contains("supervision opt exit_and_resume"));
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
    assert!(rendered.contains("supervision req 31"));
    assert!(rendered.contains("supervision ow  wrapper_background_shell"));
    assert!(rendered.contains("supervision auto false"));
    assert!(rendered.contains("supervision cl  call-31"));
    assert!(rendered.contains("supervision ob  wrapper_background_shell_streaming_output"));
    assert!(rendered.contains("supervision os  recent_output_observed"));
    assert!(rendered.contains("supervision jb  bg-1 "));
    assert!(rendered.contains("supervision cmd echo READY; sleep 2"));
    assert!(rendered.contains("supervision ln  1"));
    assert!(rendered.contains("supervision oa"));
    assert!(rendered.contains("supervision ot  READY"));
    assert!(rendered.contains("supervision opt observe_status :status"));
    assert!(rendered.contains("supervision opt interrupt_turn :interrupt"));
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
    assert!(rendered.contains("async guard act observe_or_interrupt"));
    assert!(rendered.contains("async guard pol warn_only"));
    assert!(rendered.contains("async guard auto false"));
    assert!(rendered.contains("async guard opt observe_status :status"));
    assert!(rendered.contains("async guard opt interrupt_turn :interrupt"));
}
