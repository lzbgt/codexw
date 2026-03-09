use super::super::*;

#[test]
fn model_picker_applies_selected_model_and_effort() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    state.models = extract_models(&json!({
        "data": [{
            "id": "gpt-5-codex",
            "displayName": "GPT-5 Codex",
            "description": "Flagship coding model",
            "supportsPersonality": true,
            "isDefault": true,
            "defaultReasoningLevel": "medium",
            "supportedReasoningLevels": [
                {"effort": "low", "description": "fast"},
                {"effort": "medium", "description": "balanced"},
                {"effort": "high", "description": "deep"}
            ]
        }]
    }));
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/model",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("open model picker"),
        Some(true)
    );
    assert_eq!(state.pending_selection, Some(PendingSelection::Model));

    assert_eq!(
        try_handle_prefixed_submission(
            "1",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("select model"),
        Some(true)
    );
    assert_eq!(
        state.pending_selection,
        Some(PendingSelection::ReasoningEffort {
            model_id: "gpt-5-codex".to_string(),
        })
    );

    assert_eq!(
        try_handle_prefixed_submission(
            "3",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("select reasoning effort"),
        Some(true)
    );
    assert_eq!(
        state.session_overrides.model,
        Some(Some("gpt-5-codex".to_string()))
    );
    assert_eq!(
        state.session_overrides.reasoning_effort,
        Some(Some("high".to_string()))
    );
    assert_eq!(state.pending_selection, None);

    let contents = config_contents(&codex_home);
    assert!(contents.contains("model = \"gpt-5-codex\""));
    assert!(contents.contains("model_reasoning_effort = \"high\""));
}
