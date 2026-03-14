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
    assert!(rendered.contains("async time"));
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
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(65),
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
fn async_tool_supervision_classifies_slow_and_wedged_elapsed_time() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "slow".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(20),
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

    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "wedged".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(65),
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
}
