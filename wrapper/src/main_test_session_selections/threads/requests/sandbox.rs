use super::super::super::*;

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
    let (tx, _rx) = std::sync::mpsc::channel();
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
        &tx,
        &mut start_after_initialize,
    )
    .expect("process notification");

    let contents = config_contents(&codex_home);
    assert!(contents.contains("[windows]"));
    assert!(contents.contains("sandbox = \"elevated\""));
}
