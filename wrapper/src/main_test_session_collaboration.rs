use crate::collaboration_preset::CollaborationModePreset;
use crate::collaboration_preset::extract_collaboration_mode_presets;
use crate::collaboration_view::render_collaboration_modes;
use crate::collaboration_view::summarize_active_collaboration_mode;
use crate::session_prompt_status::render_prompt_status;
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
