use super::*;

#[test]
fn ctrl_c_preserves_draft_while_interrupting_active_turn() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());

    let mut editor = LineEditor::default();
    editor.insert_str("first\nsecond");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result =
        handle_ctrl_c(&mut state, &mut editor, &mut output, &mut writer, "/tmp").expect("ctrl-c");
    assert!(result.is_none());
    assert_eq!(editor.buffer(), "first\nsecond");
}

#[test]
fn escape_preserves_draft_while_interrupting_active_turn() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());

    let mut editor = LineEditor::default();
    editor.insert_str("first\nsecond");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result =
        handle_escape(&mut state, &mut editor, &mut output, &mut writer, true).expect("escape");
    assert!(result.is_none());
    assert_eq!(editor.buffer(), "first\nsecond");
}

#[test]
fn idle_ctrl_c_exit_marks_resume_hint_as_emitted_when_thread_exists() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());

    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result = handle_ctrl_c(
        &mut state,
        &mut editor,
        &mut output,
        &mut writer,
        "/tmp/project",
    )
    .expect("ctrl-c");

    assert_eq!(result, Some(false));
    assert!(state.resume_exit_hint_emitted);
}

#[test]
fn submit_is_ignored_while_local_command_is_active() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.active_exec_process_id = Some("proc-1".to_string());

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
        prompt: Vec::new(),
    });

    let mut editor = LineEditor::default();
    editor.insert_str("should stay buffered");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let continue_running = handle_submit(
        &cli,
        "/tmp",
        &mut state,
        &mut editor,
        &mut output,
        &mut writer,
    )
    .expect("submit");

    assert!(continue_running);
    assert_eq!(editor.buffer(), "should stay buffered");
    assert_eq!(editor.history.len(), 0);
}

#[test]
fn startup_resume_picker_accepts_bare_numeric_selection() {
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: true,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        prompt: Vec::new(),
    });
    let mut state = AppState::new(true, false);
    state.startup_resume_picker = true;
    state.last_listed_thread_ids = vec!["thread-99".to_string()];
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let handled = try_handle_prefixed_submission(
        "1",
        &cli,
        "/tmp",
        &mut state,
        &mut editor,
        &mut output,
        &mut writer,
    )
    .expect("submission");

    assert_eq!(handled, Some(true));
    assert!(state.pending_thread_switch);
    assert!(state.pending.values().any(|pending| matches!(
        pending,
        PendingRequest::ResumeThread {
            initial_prompt: None
        }
    )));
}
