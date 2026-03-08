use crate::catalog_views::extract_file_search_paths;
use crate::catalog_views::extract_thread_ids;
use crate::catalog_views::render_fuzzy_file_search_results;
use crate::catalog_views::render_thread_list;
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
