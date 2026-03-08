use crate::catalog_views::extract_file_search_paths;
use crate::catalog_views::extract_thread_ids;
use crate::catalog_views::render_apps_list;
use crate::catalog_views::render_experimental_features_list;
use crate::catalog_views::render_fuzzy_file_search_results;
use crate::catalog_views::render_models_list;
use crate::catalog_views::render_thread_list;
use crate::commands::builtin_command_names;
use crate::commands::builtin_help_lines;
use crate::commands::quote_if_needed;
use crate::commands_completion::render_slash_completion_candidates;
use crate::history::latest_conversation_history_items;
use crate::history::seed_resumed_state_from_turns;
use crate::input::AppCatalogEntry;
use crate::state::AppState;
use crate::status_views::render_rate_limit_lines;
use serde_json::Value;
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
fn resume_helpers_only_keep_recent_conversation_context() {
    let turns = vec![
        json!({
            "items": [
                {"type": "userMessage", "content": [{"type": "text", "text": "old objective"}]},
                {"type": "agentMessage", "text": "old reply"},
                {"type": "reasoning", "text": "ignore"}
            ]
        }),
        json!({
            "items": [
                {"type": "userMessage", "content": [{"type": "text", "text": "latest request"}]},
                {"type": "agentMessage", "text": "latest reply"}
            ]
        }),
    ];

    let mut state = AppState::new(true, false);
    seed_resumed_state_from_turns(&turns, &mut state);
    assert_eq!(state.objective.as_deref(), Some("latest request"));
    assert_eq!(state.last_agent_message.as_deref(), Some("latest reply"));

    let recent_items = latest_conversation_history_items(&turns, 2);
    assert_eq!(recent_items.len(), 2);
    assert_eq!(
        recent_items[0].get("type").and_then(Value::as_str),
        Some("userMessage")
    );
    assert_eq!(
        recent_items[1].get("type").and_then(Value::as_str),
        Some("agentMessage")
    );
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
    assert_eq!(quote_if_needed("src/main.rs"), "src/main.rs");
    assert_eq!(
        quote_if_needed("path with spaces.rs"),
        "\"path with spaces.rs\""
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
fn slash_completion_rendering_includes_descriptions() {
    let rendered = render_slash_completion_candidates("re", &["resume", "review"], false);
    assert!(rendered.contains("/resume"));
    assert!(rendered.contains("resume a saved thread"));
    assert!(rendered.contains("/review"));
    assert!(rendered.contains("review current changes and find issues"));
}

#[test]
fn bare_slash_completion_uses_native_like_order() {
    let rendered = render_slash_completion_candidates("", builtin_command_names(), false);
    let review_index = rendered.find("/review").expect("review");
    let rename_index = rendered.find("/rename").expect("rename");
    let new_index = rendered.find("/new").expect("new");
    assert!(review_index < rename_index);
    assert!(rename_index < new_index);
}

#[test]
fn help_lines_are_derived_from_command_metadata() {
    let rendered = builtin_help_lines().join("\n");
    assert!(rendered.contains(":resume [thread-id|n]"));
    assert!(rendered.contains("resume a saved thread"));
    assert!(rendered.contains(":feedback <category> [reason] [--logs|--no-logs]"));
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
fn rate_limit_lines_show_remaining_capacity_and_reset() {
    let rendered = render_rate_limit_lines(Some(&json!({
        "primary": {
            "usedPercent": 25,
            "windowDurationMins": 300,
            "resetsAt": "2026-03-08T14:30:00Z"
        },
        "secondary": {
            "usedPercent": 40,
            "windowDurationMins": 10080,
            "resetsAt": "2026-03-10T09:00:00Z"
        }
    })));
    let rendered = rendered.join("\n");
    assert!(rendered.contains("5h limit 75% left"));
    assert!(rendered.contains("weekly limit 60% left"));
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
