use crate::Cli;
use crate::catalog_file_search::extract_file_search_paths;
use crate::catalog_file_search::render_fuzzy_file_search_results;
use crate::catalog_thread_list::extract_thread_ids;
use crate::catalog_thread_list::render_thread_list;
use crate::catalog_thread_list::should_fallback_to_all_workspaces;
use crate::catalog_thread_list::thread_list_is_empty;
use crate::events::process_server_line;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::requests::thread_list_params;
use crate::rpc::RequestId;
use crate::state::AppState;
use serde_json::json;
use std::process::Command;
use std::process::Stdio;

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
    let workspace_scoped = thread_list_params(Some("/tmp/project"), None);
    assert_eq!(workspace_scoped["cwd"], "/tmp/project");

    let all_workspaces = thread_list_params(None, None);
    assert!(all_workspaces.get("cwd").is_none());
}

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

fn test_cli() -> Cli {
    Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
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
    }
}

#[test]
fn empty_workspace_resume_list_retries_without_cwd_filter() {
    let cli = test_cli();
    let mut state = AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    let mut start_after_initialize = None;
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ListThreads {
            search_term: None,
            cwd_filter: Some("/tmp/project".to_string()),
        },
    );

    process_server_line(
        serde_json::to_string(&json!({
            "id": request_id,
            "result": {"data": []}
        }))
        .expect("serialize response"),
        &cli,
        "/tmp/project",
        &mut state,
        &mut output,
        &mut writer,
        &mut start_after_initialize,
    )
    .expect("process response");

    assert_eq!(state.pending.len(), 1);
    let (next_id, pending) = state
        .pending
        .iter()
        .next()
        .expect("pending fallback request");
    assert_eq!(next_id, &RequestId::Integer(2));
    match pending {
        PendingRequest::ListThreads {
            search_term,
            cwd_filter,
        } => {
            assert_eq!(search_term, &None);
            assert_eq!(cwd_filter, &None);
        }
        other => panic!("expected fallback list request, got {other:?}"),
    }
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
