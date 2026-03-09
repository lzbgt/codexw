use super::*;

#[test]
fn init_command_starts_new_thread_with_upstream_prompt() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let workspace = tempfile::tempdir().expect("tempdir");
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/init",
            &cli,
            workspace.path().to_str().expect("workspace path"),
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run init"),
        Some(true)
    );

    let pending = state.pending.values().next().expect("pending request");
    match pending {
        PendingRequest::StartThread { initial_prompt } => {
            assert_eq!(initial_prompt.as_deref(), Some(INIT_PROMPT.trim_end()));
        }
        other => panic!("expected StartThread, got {other:?}"),
    }
}

#[test]
fn init_command_uses_turn_start_when_thread_exists() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    let workspace = tempfile::tempdir().expect("tempdir");
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/init",
            &cli,
            workspace.path().to_str().expect("workspace path"),
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run init"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.first().expect("turn/start request");
    assert_eq!(request["method"], json!("turn/start"));
    assert_eq!(request["params"]["threadId"], json!("thread-1"));
    assert_eq!(request["params"]["input"][0]["type"], json!("text"));
    assert_eq!(
        request["params"]["input"][0]["text"],
        json!(INIT_PROMPT.trim_end())
    );
}

#[test]
fn init_command_skips_existing_agents_file() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("AGENTS.md"), "existing").expect("write AGENTS");
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/init",
            &cli,
            workspace.path().to_str().expect("workspace path"),
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run init"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    assert!(requests.is_empty());
    assert!(state.pending.is_empty());
}

#[test]
fn agent_command_requests_filtered_agent_threads() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/agent",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run agent command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/list");
    assert_eq!(request["params"]["cwd"], "/tmp/project");
    assert_eq!(
        request["params"]["sourceKinds"],
        json!(["subAgentThreadSpawn"])
    );
}

#[test]
fn multi_agents_command_tracks_agent_thread_view() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/multi-agents",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run multi-agents command"),
        Some(true)
    );

    let pending = state.pending.values().next().expect("pending request");
    match pending {
        PendingRequest::ListThreads {
            source_kinds, view, ..
        } => {
            assert_eq!(source_kinds, &Some(vec!["subAgentThreadSpawn".to_string()]));
            assert_eq!(view, &ThreadListView::Agents);
        }
        other => panic!("expected agent thread list request, got {other:?}"),
    }
}

#[test]
fn new_thread_requests_advertise_client_dynamic_tools() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/new",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run new command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/start");
    let names = request["params"]["dynamicTools"]
        .as_array()
        .expect("dynamic tools")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "orchestration_status",
            "orchestration_list_workers",
            "orchestration_suggest_actions",
            "orchestration_list_dependencies",
            "workspace_list_dir",
            "workspace_stat_path",
            "workspace_read_file",
            "workspace_find_files",
            "workspace_search_text",
            "background_shell_start",
            "background_shell_poll",
            "background_shell_send",
            "background_shell_set_alias",
            "background_shell_list_capabilities",
            "background_shell_list_services",
            "background_shell_update_service",
            "background_shell_update_dependencies",
            "background_shell_inspect_capability",
            "background_shell_attach",
            "background_shell_wait_ready",
            "background_shell_invoke_recipe",
            "background_shell_list",
            "background_shell_terminate",
            "background_shell_clean"
        ]
    );
}

#[test]
fn new_thread_omits_dynamic_tools_when_experimental_api_is_disabled() {
    let mut cli = build_cli();
    cli.no_experimental_api = true;
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/new",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run new command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/start");
    assert!(request["params"].get("dynamicTools").is_none());
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn new_thread_requests_persist_extended_history() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/new",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run new command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/start");
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn resume_requests_persist_extended_history() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/resume thread-77",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run resume command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/resume");
    assert_eq!(request["params"]["threadId"], "thread-77");
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn fork_requests_persist_extended_history() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-77".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/fork",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run fork command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/fork");
    assert_eq!(request["params"]["threadId"], "thread-77");
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn windows_sandbox_setup_request_targets_workspace() {
    let mut state = AppState::new(true, false);
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    send_windows_sandbox_setup_start(&mut writer, &mut state, "/tmp/project", "elevated")
        .expect("send setup request");

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "windowsSandbox/setupStart");
    assert_eq!(request["params"]["mode"], "elevated");
    assert_eq!(request["params"]["cwd"], "/tmp/project");
}

#[test]
fn setup_default_sandbox_is_scoped_to_windows() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/setup-default-sandbox",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run setup-default-sandbox"),
        Some(true)
    );

    if cfg!(target_os = "windows") {
        let pending = state.pending.values().next().expect("pending request");
        match pending {
            PendingRequest::WindowsSandboxSetupStart { mode } => {
                assert_eq!(mode, "elevated");
            }
            other => panic!("expected windows sandbox setup request, got {other:?}"),
        }
    } else {
        assert!(state.pending.is_empty());
    }
}

#[test]
fn windows_sandbox_setup_completed_persists_mode() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    let mut start_after_initialize = None;

    process_server_line(
        serde_json::to_string(&json!({
            "method": "windowsSandbox/setupCompleted",
            "params": {
                "mode": "elevated",
                "success": true,
                "error": null
            }
        }))
        .expect("serialize notification"),
        &cli,
        "/tmp/project",
        &mut state,
        &mut output,
        &mut writer,
        &mut start_after_initialize,
    )
    .expect("process notification");

    let contents = config_contents(&codex_home);
    assert!(contents.contains("[windows]"));
    assert!(contents.contains("sandbox = \"elevated\""));
}
