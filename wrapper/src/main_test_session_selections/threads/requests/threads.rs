use super::super::super::*;

#[test]
fn new_thread_does_not_send_removed_dynamic_tools_param() {
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
    assert!(request["params"].get("dynamicTools").is_none());
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
