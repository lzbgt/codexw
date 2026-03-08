use crate::Cli;
use crate::collaboration::CollaborationModePreset;
use crate::collaboration::extract_collaboration_mode_presets;
use crate::collaboration::render_collaboration_modes;
use crate::collaboration::summarize_active_collaboration_mode;
use crate::model_session::extract_models;
use crate::model_session::render_personality_options;
use crate::model_session::summarize_active_personality;
use crate::session_prompt_status::render_prompt_status;
use crate::session_snapshot::render_status_snapshot;
use crate::transcript_summary::summarize_thread_status_for_display;
use serde_json::json;

#[test]
fn collaboration_modes_are_extractable_from_response() {
    let presets = extract_collaboration_mode_presets(&json!({
        "data": [
            {
                "name": "Plan",
                "mode": "plan",
                "model": "gpt-5-codex",
                "reasoning_effort": "high"
            },
            {
                "name": "Default",
                "mode": "default",
                "model": null,
                "reasoning_effort": null
            }
        ]
    }));
    assert_eq!(presets.len(), 2);
    assert_eq!(presets[0].name, "Plan");
    assert_eq!(presets[0].mode_kind.as_deref(), Some("plan"));
    assert_eq!(presets[1].name, "Default");
}

#[test]
fn collaboration_mode_rendering_shows_current_and_available_presets() {
    let presets = vec![
        CollaborationModePreset {
            name: "Default".to_string(),
            mode_kind: Some("default".to_string()),
            model: None,
            reasoning_effort: None,
        },
        CollaborationModePreset {
            name: "Plan".to_string(),
            mode_kind: Some("plan".to_string()),
            model: Some("gpt-5-codex".to_string()),
            reasoning_effort: Some(Some("high".to_string())),
        },
    ];
    let mut state = crate::state::AppState::new(true, false);
    state.collaboration_modes = presets;
    state.active_collaboration_mode = state.collaboration_modes.get(1).cloned();
    let rendered = render_collaboration_modes(&state);
    assert!(rendered.contains("current         Plan"));
    assert!(rendered.contains("available"));
    assert!(rendered.contains("mode=plan"));
    assert!(rendered.contains("model=gpt-5-codex"));
}

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
fn thread_status_summary_prefers_human_flags() {
    assert_eq!(
        summarize_thread_status_for_display(&json!({
            "status": {"type": "active", "activeFlags": ["waitingOnApproval"]}
        })),
        Some("waiting on approval".to_string())
    );
    assert_eq!(
        summarize_thread_status_for_display(&json!({
            "status": {"type": "idle", "activeFlags": []}
        })),
        Some("ready".to_string())
    );
}

#[test]
fn prompt_status_uses_active_detail_when_present() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.started_turn_count = 2;
    state.last_status_line = Some("waiting on approval".to_string());
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("waiting on approval"));
}

#[test]
fn prompt_status_mentions_plan_mode_when_selected() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_collaboration_mode = Some(CollaborationModePreset {
        name: "Plan".to_string(),
        mode_kind: Some("plan".to_string()),
        model: Some("gpt-5-codex".to_string()),
        reasoning_effort: Some(Some("high".to_string())),
    });
    assert_eq!(
        summarize_active_collaboration_mode(&state),
        "Plan (mode=plan, model=gpt-5-codex, effort=high)"
    );
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("plan mode"));
}

#[test]
fn prompt_status_mentions_personality_when_selected() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_personality = Some("friendly".to_string());
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("Friendly"));
}

#[test]
fn prompt_status_mentions_realtime_when_active() {
    let mut state = crate::state::AppState::new(true, false);
    state.realtime_active = true;
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("realtime"));
}

#[test]
fn status_snapshot_includes_realtime_fields() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.realtime_active = true;
    state.realtime_session_id = Some("rt-1".to_string());
    state.realtime_prompt = Some("hello world".to_string());
    state.realtime_last_error = Some("bad gateway".to_string());
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
    assert!(rendered.contains("realtime        true"));
    assert!(rendered.contains("realtime id     rt-1"));
    assert!(rendered.contains("realtime prompt hello world"));
    assert!(rendered.contains("realtime error  bad gateway"));
}
