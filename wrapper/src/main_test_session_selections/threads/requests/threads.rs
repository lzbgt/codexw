use super::super::super::*;
use serde_json::Value;

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
