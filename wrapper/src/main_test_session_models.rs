use crate::Cli;
use crate::model_session::extract_models;
use crate::model_session::render_personality_options;
use crate::model_session::summarize_active_personality;
use crate::session_snapshot::render_status_snapshot;
use serde_json::json;

#[test]
fn models_are_extractable_with_personality_support() {
    let models = extract_models(&json!({
        "data": [
            {
                "id": "gpt-5-codex",
                "displayName": "GPT-5 Codex",
                "supportsPersonality": true,
                "isDefault": true
            },
            {
                "id": "legacy-model",
                "displayName": "Legacy",
                "supportsPersonality": false,
                "isDefault": false
            }
        ]
    }));
    assert_eq!(models.len(), 2);
    assert!(models[0].supports_personality);
    assert!(!models[1].supports_personality);
}

#[test]
fn personality_rendering_shows_current_and_model_support() {
    let mut state = crate::state::AppState::new(true, false);
    state.models = extract_models(&json!({
        "data": [
            {
                "id": "gpt-5-codex",
                "displayName": "GPT-5 Codex",
                "supportsPersonality": true,
                "isDefault": true
            }
        ]
    }));
    state.active_personality = Some("pragmatic".to_string());
    let cli = Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
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
    };
    let rendered = render_personality_options(&cli, &state);
    assert_eq!(summarize_active_personality(&state), "Pragmatic");
    assert!(rendered.contains("current          Pragmatic"));
    assert!(rendered.contains("current model     GPT-5 Codex [supports personality]"));
}

#[test]
fn status_snapshot_surfaces_effective_model_personality_support() {
    let mut state = crate::state::AppState::new(true, false);
    state.models = extract_models(&json!({
        "data": [
            {
                "id": "gpt-5-codex",
                "displayName": "GPT-5 Codex",
                "supportsPersonality": true,
                "isDefault": true
            }
        ]
    }));
    let cli = crate::runtime::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
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
    let rendered = render_status_snapshot(&cli, "/tmp/project", &state);
    assert!(rendered.contains("model           GPT-5 Codex [supports personality]"));
    assert!(rendered.contains("models cached   1"));
}

#[test]
fn prompt_status_mentions_personality_when_selected() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_personality = Some("friendly".to_string());
    let rendered = crate::session_prompt_status::render_prompt_status(&state);
    assert!(rendered.contains("Friendly"));
}
