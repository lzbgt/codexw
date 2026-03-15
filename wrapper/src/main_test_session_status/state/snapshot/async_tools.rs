use super::*;

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
