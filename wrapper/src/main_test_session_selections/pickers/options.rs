use super::super::*;

#[test]
fn permissions_picker_updates_approval_and_sandbox_overrides() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/permissions",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("open permissions picker"),
        Some(true)
    );
    assert_eq!(state.pending_selection, Some(PendingSelection::Permissions));

    assert_eq!(
        try_handle_prefixed_submission(
            "2",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("select permissions preset"),
        Some(true)
    );
    assert_eq!(
        state.session_overrides.approval_policy.as_deref(),
        Some("on-request")
    );
    assert_eq!(
        state.session_overrides.thread_sandbox_mode.as_deref(),
        Some("workspace-write")
    );
    assert_eq!(state.pending_selection, None);
}

#[test]
fn fast_command_toggles_service_tier_override() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/fast",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("enable fast"),
        Some(true)
    );
    assert_eq!(
        state.session_overrides.service_tier,
        Some(Some("fast".to_string()))
    );
    assert!(config_contents(&codex_home).contains("service_tier = \"fast\""));

    assert_eq!(
        try_handle_prefixed_submission(
            "/fast",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("disable fast"),
        Some(true)
    );
    assert_eq!(state.session_overrides.service_tier, Some(None));
    assert!(!config_contents(&codex_home).contains("service_tier = "));
}

#[test]
fn personality_command_persists_selected_personality() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    state.models = extract_models(&json!({
        "data": [{
            "id": "gpt-5-codex",
            "displayName": "GPT-5 Codex",
            "supportsPersonality": true,
            "isDefault": true
        }]
    }));
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/personality friendly",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("set personality"),
        Some(true)
    );

    assert_eq!(
        state.session_overrides.personality,
        Some(Some("friendly".to_string()))
    );
    assert_eq!(state.active_personality.as_deref(), Some("friendly"));
    assert!(config_contents(&codex_home).contains("personality = \"friendly\""));
}

#[test]
fn theme_command_persists_selected_theme() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/theme base16-ocean.dark",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("set theme"),
        Some(true)
    );

    let contents = config_contents(&codex_home);
    assert!(contents.contains("[tui]"));
    assert!(contents.contains("theme = \"base16-ocean.dark\""));
}
