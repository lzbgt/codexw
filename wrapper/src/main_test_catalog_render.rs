use crate::catalog_views::render_apps_list;
use crate::catalog_views::render_experimental_features_list;
use crate::catalog_views::render_models_list;
use crate::input::AppCatalogEntry;
use serde_json::json;

#[test]
fn app_list_rendering_includes_slug_and_status() {
    let rendered = render_apps_list(&[
        AppCatalogEntry {
            id: "connector_1".to_string(),
            name: "Demo App".to_string(),
            slug: "demo-app".to_string(),
            enabled: true,
        },
        AppCatalogEntry {
            id: "connector_2".to_string(),
            name: "Hidden App".to_string(),
            slug: "hidden-app".to_string(),
            enabled: false,
        },
    ]);
    assert!(rendered.contains("$demo-app"));
    assert!(rendered.contains("[enabled]"));
    assert!(rendered.contains("[disabled]"));
}

#[test]
fn experimental_feature_rendering_shows_stage_status_and_key() {
    let rendered = render_experimental_features_list(&json!({
        "data": [
            {
                "name": "background-terminals",
                "displayName": "Background terminals",
                "enabled": true,
                "stage": "beta",
                "description": "Run background terminal sessions",
                "announcement": "This feature is in beta"
            }
        ]
    }));
    assert!(rendered.contains("Background terminals"));
    assert!(rendered.contains("[enabled]"));
    assert!(rendered.contains("[beta]"));
    assert!(rendered.contains("key: background-terminals"));
    assert!(rendered.contains("Run background terminal sessions"));
}

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
