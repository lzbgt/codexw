use crate::model_catalog::extract_models;
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
