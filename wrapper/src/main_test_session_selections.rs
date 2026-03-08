use std::process::Command;
use std::process::Stdio;

use serde_json::json;

use crate::Cli;
use crate::dispatch_submit_commands::try_handle_prefixed_submission;
use crate::editor::LineEditor;
use crate::model_catalog::extract_models;
use crate::output::Output;
use crate::state::AppState;
use crate::state::PendingSelection;

fn build_cli() -> Cli {
    crate::runtime_process::normalize_cli(Cli {
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
    })
}

fn spawn_sink_stdin() -> std::process::ChildStdin {
    Command::new("sh")
        .arg("-c")
        .arg("cat >/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sink")
        .stdin
        .take()
        .expect("stdin")
}

#[test]
fn model_picker_applies_selected_model_and_effort() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
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
}

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
}
