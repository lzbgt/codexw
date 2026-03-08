use crate::catalog_file_search::extract_file_search_paths;
use crate::catalog_file_search::render_fuzzy_file_search_results;
use crate::catalog_thread_list::extract_thread_ids;
use crate::catalog_thread_list::render_thread_list;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::response_bootstrap_catalog_views::handle_threads_listed;
use crate::state::AppState;
use serde_json::json;
use std::process::Command;
use std::process::Stdio;

fn spawn_sink_stdin() -> std::process::ChildStdin {
    Command::new("sh")
        .arg("-c")
        .arg("cat >/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sink")
        .stdin
        .take()
        .expect("stdin")
}

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
    let rendered = render_thread_list(&result, None);
    assert!(rendered.contains(" 1. thr_newer"));
    assert!(rendered.contains(" 2. thr_older"));
    assert!(rendered.contains("Use /resume <n>"));
    assert_eq!(extract_thread_ids(&result), vec!["thr_newer", "thr_older"]);
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
fn startup_resume_picker_retries_without_cwd_when_exact_match_is_empty() {
    let mut state = AppState::new(true, false);
    state.startup_resume_picker = true;
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    handle_threads_listed(
        &json!({"data": []}),
        None,
        Some("/tmp/workspace"),
        true,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("handle threads");

    assert!(state.last_listed_thread_ids.is_empty());
    assert!(state.pending.values().any(|pending| matches!(
        pending,
        PendingRequest::ListThreads {
            search_term: None,
            cwd_filter: None,
            allow_fallback_all: false,
        }
    )));
}
