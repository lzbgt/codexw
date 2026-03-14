use crate::Cli;
use crate::events::process_server_line;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::requests::ThreadListView;
use crate::rpc::RequestId;
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
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    }
}

#[test]
fn empty_workspace_resume_list_retries_without_cwd_filter() {
    let cli = test_cli();
    let mut state = AppState::new(true, false);
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    let mut start_after_initialize = None;
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ListThreads {
            search_term: None,
            cwd_filter: Some("/tmp/project".to_string()),
            source_kinds: None,
            view: ThreadListView::Threads,
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
        &tx,
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
            source_kinds,
            view,
        } => {
            assert_eq!(search_term, &None);
            assert_eq!(cwd_filter, &None);
            assert_eq!(source_kinds, &None);
            assert_eq!(view, &ThreadListView::Threads);
        }
        other => panic!("expected fallback list request, got {other:?}"),
    }
}
