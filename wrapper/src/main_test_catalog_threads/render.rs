use crate::catalog_file_search::extract_file_search_paths;
use crate::catalog_file_search::render_fuzzy_file_search_results;
use crate::catalog_thread_list::extract_thread_ids;
use crate::catalog_thread_list::render_thread_list;
use crate::catalog_thread_list::should_fallback_to_all_workspaces;
use crate::catalog_thread_list::thread_list_is_empty;
use crate::catalog_thread_list::thread_list_snapshot;
use crate::requests::ThreadListView;
use crate::requests::thread_list_params;
use serde_json::json;

#[test]
fn thread_list_is_numbered_and_extractable() {
    let result = json!({
        "data": [
            {
                "id": "thr_older",
                "preview": "older thread",
                "status": {"type": "idle"},
                "updatedAt": 1
            },
            {
                "id": "thr_newer",
                "preview": "newer thread",
                "status": {"type": "active"},
                "updatedAt": 2
            }
        ]
    });
    let rendered = render_thread_list(&result, None, ThreadListView::Threads);
    let snapshot = thread_list_snapshot(&result);
    assert!(rendered.contains(" 1. thr_newer"));
    assert!(rendered.contains(" 2. thr_older"));
    assert!(rendered.contains("Use /resume <n>"));
    assert_eq!(extract_thread_ids(&result), vec!["thr_newer", "thr_older"]);
    assert_eq!(snapshot.thread_ids(), vec!["thr_newer", "thr_older"]);
    assert!(!thread_list_is_empty(&result));
}

#[test]
fn empty_workspace_thread_list_falls_back_to_all_workspaces() {
    let empty = json!({ "data": [] });
    assert!(thread_list_is_empty(&empty));
    assert!(should_fallback_to_all_workspaces(
        &empty,
        None,
        Some("/tmp/project")
    ));
    assert!(!should_fallback_to_all_workspaces(
        &empty,
        Some("search"),
        Some("/tmp/project")
    ));
    assert!(!should_fallback_to_all_workspaces(&empty, None, None));
}

#[test]
fn all_workspace_thread_list_request_omits_cwd_filter() {
    let workspace_scoped = thread_list_params(Some("/tmp/project"), None, None);
    assert_eq!(workspace_scoped["cwd"], "/tmp/project");

    let all_workspaces = thread_list_params(None, None, None);
    assert!(all_workspaces.get("cwd").is_none());
}

#[test]
fn agent_thread_list_rendering_uses_agent_wording() {
    let result = json!({
        "data": [
            {
                "id": "agent_thr",
                "preview": "agent work",
                "status": {"type": "idle"},
                "updatedAt": 2
            }
        ]
    });
    let rendered = render_thread_list(&result, None, ThreadListView::Agents);
    assert!(rendered.contains(" 1. agent_thr"));
    assert!(rendered.contains("Use /resume <n> to switch to one of these agent threads."));
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
