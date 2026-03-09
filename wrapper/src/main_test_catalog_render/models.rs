use crate::catalog_backend_views::render_models_list;
use serde_json::json;

#[test]
fn models_render_default_and_personality_support_markers() {
    let rendered = render_models_list(&json!({
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
    assert!(rendered.contains("GPT-5 Codex (gpt-5-codex) [default] [supports personality]"));
    assert!(rendered.contains("Legacy (legacy-model) [personality unsupported]"));
}
