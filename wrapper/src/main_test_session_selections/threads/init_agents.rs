use super::super::*;

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
