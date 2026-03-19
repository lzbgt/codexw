use super::*;

#[test]
fn ctrl_c_clears_visible_draft_before_interrupting_active_turn() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());

    let mut editor = LineEditor::default();
    editor.insert_str("first\nsecond");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result = handle_ctrl_c(
        &mut state,
        &mut editor,
        &mut output,
        &mut writer,
        "/tmp",
        true,
    )
    .expect("ctrl-c");
    assert!(result.is_none());
    assert_eq!(editor.buffer(), "");
    assert!(state.pending.is_empty());
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
fn repeated_ctrl_c_exits_when_turn_interrupt_is_already_pending() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id, PendingRequest::InterruptTurn);

    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result = handle_ctrl_c(
        &mut state,
        &mut editor,
        &mut output,
        &mut writer,
        "/tmp/project",
        true,
    )
    .expect("ctrl-c");

    assert_eq!(result, Some(false));
    assert_eq!(editor.buffer(), "");
    assert!(state.resume_exit_hint_emitted);
}

#[test]
fn repeated_escape_exits_when_turn_interrupt_is_already_pending() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id, PendingRequest::InterruptTurn);

    let mut editor = LineEditor::default();
    editor.insert_str("first\nsecond");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result =
        handle_escape(&mut state, &mut editor, &mut output, &mut writer, true).expect("escape");

    assert_eq!(result, Some(false));
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
        true,
    )
    .expect("ctrl-c");

    assert_eq!(result, Some(false));
    assert!(state.resume_exit_hint_emitted);
}

#[test]
fn idle_ctrl_c_clears_visible_draft_without_exiting() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());

    let mut editor = LineEditor::default();
    editor.insert_str("draft text");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result = handle_ctrl_c(
        &mut state,
        &mut editor,
        &mut output,
        &mut writer,
        "/tmp/project",
        true,
    )
    .expect("ctrl-c");

    assert!(result.is_none());
    assert_eq!(editor.buffer(), "");
    assert!(!state.resume_exit_hint_emitted);
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
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
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
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
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

#[test]
fn quiet_turn_submit_queues_prompt_for_resume_instead_of_steering() {
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
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());
    state.activity_started_at =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(120));
    state.last_server_event_at = Some(
        std::time::Instant::now()
            - crate::state::AppState::TURN_IDLE_WARNING_THRESHOLD
            - std::time::Duration::from_secs(5),
    );
    state
        .pending_local_images
        .push("/tmp/queued.png".to_string());
    state
        .pending_remote_images
        .push("https://example.com/queued.png".to_string());

    let mut editor = LineEditor::default();
    editor.insert_str("continue after self-heal");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let continue_running = handle_submit(
        &cli,
        "/tmp/project",
        &mut state,
        &mut editor,
        &mut output,
        &mut writer,
    )
    .expect("submit");

    assert!(continue_running);
    assert_eq!(
        state.staged_resume_prompt.as_deref(),
        Some("continue after self-heal")
    );
    assert!(
        state
            .pending
            .values()
            .any(|pending| matches!(pending, PendingRequest::InterruptTurn))
    );
    assert!(
        !state
            .pending
            .values()
            .any(|pending| matches!(pending, PendingRequest::SteerTurn { .. }))
    );
    assert_eq!(
        state.pending_local_images,
        vec!["/tmp/queued.png".to_string()]
    );
    assert_eq!(
        state.pending_remote_images,
        vec!["https://example.com/queued.png".to_string()]
    );
}
