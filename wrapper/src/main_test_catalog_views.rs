use crate::catalog_views::extract_file_search_paths;
use crate::catalog_views::extract_thread_ids;
use crate::catalog_views::render_apps_list;
use crate::catalog_views::render_experimental_features_list;
use crate::catalog_views::render_fuzzy_file_search_results;
use crate::catalog_views::render_models_list;
use crate::catalog_views::render_thread_list;
use crate::input::AppCatalogEntry;
use serde_json::json;

#[test]
fn thread_list_is_numbered_and_extractable() {
    let result = json!({
        "data": [
            {
                "id": "thr_1",
                "preview": "first thread",
                "status": {"type": "idle"},
                "updatedAt": 1
            },
            {
                "id": "thr_2",
                "preview": "second thread",
                "status": {"type": "active"},
                "updatedAt": 2
            }
        ]
    });
    let rendered = render_thread_list(&result, None);
    assert!(rendered.contains(" 1. thr_1"));
    assert!(rendered.contains("Use /resume <n>"));
    assert_eq!(extract_thread_ids(&result), vec!["thr_1", "thr_2"]);
}

#[test]
fn file_search_paths_are_extractable_for_numeric_insert() {
    let files = vec![
        json!({"path": "src/main.rs", "score": 1}),
        json!({"path": "src/lib.rs", "score": 2}),
    ];
    assert_eq!(
        extract_file_search_paths(&files),
        vec!["src/main.rs", "src/lib.rs"]
    );
}

#[test]
fn fuzzy_file_search_rendering_shows_ranked_paths() {
    let rendered = render_fuzzy_file_search_results(
        "agent",
        &[
            json!({"path": "src/agent.rs", "score": 99}),
            json!({"path": "tests/agent_test.rs", "score": 78}),
        ],
    );
    assert!(rendered.contains("Query: agent"));
    assert!(rendered.contains("1. src/agent.rs  [score 99]"));
    assert!(rendered.contains("2. tests/agent_test.rs  [score 78]"));
}

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
