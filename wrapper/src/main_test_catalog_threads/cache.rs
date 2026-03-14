use std::process::Command;
use std::process::Stdio;

use crate::Cli;
use crate::catalog_thread_list::ThreadListEntry;
use crate::catalog_thread_list::ThreadListSnapshot;
use crate::dispatch_command_thread_navigation_session::try_handle_thread_session_navigation;
use crate::output::Output;
use crate::recent_thread_cache::persist_recent_thread_snapshot;
use crate::requests::PendingRequest;
use crate::requests::ThreadListView;
use crate::response_bootstrap_init::handle_initialize_success;
use crate::runtime_process::StartMode;
use crate::runtime_process::StartupThreadAction;
use crate::state::AppState;

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
    crate::runtime_process::normalize_cli(Cli {
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
    })
}

fn cache_snapshot() -> ThreadListSnapshot {
    ThreadListSnapshot::from_entries(vec![
        ThreadListEntry {
            id: "thr-old".to_string(),
            preview: "older".to_string(),
            status: "idle".to_string(),
            updated_at: Some(1),
        },
        ThreadListEntry {
            id: "thr-new".to_string(),
            preview: "newer".to_string(),
            status: "active".to_string(),
            updated_at: Some(2),
        },
    ])
}

#[test]
fn resume_without_args_uses_cached_recent_threads_before_live_list_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_home = temp.path().join("codex-home");
    persist_recent_thread_snapshot(Some(&codex_home), &cache_snapshot()).expect("persist cache");

    let cli = test_cli();
    let mut state = AppState::new(true, false);
    state.codex_home_override = Some(codex_home);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let handled = try_handle_thread_session_navigation(
        "resume",
        &[],
        &cli,
        "/tmp/project",
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("handle resume")
    .expect("handled");

    assert!(handled);
    assert_eq!(state.last_listed_thread_ids, vec!["thr-new", "thr-old"]);
    assert_eq!(state.pending.len(), 1);
    assert!(matches!(
        state.pending.values().next(),
        Some(PendingRequest::ListThreads {
            search_term: None,
            cwd_filter: Some(cwd),
            source_kinds: None,
            view: ThreadListView::Threads,
        }) if cwd == "/tmp/project"
    ));
}

#[test]
fn startup_resume_picker_uses_cached_recent_threads_before_live_list_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_home = temp.path().join("codex-home");
    persist_recent_thread_snapshot(Some(&codex_home), &cache_snapshot()).expect("persist cache");

    let cli = test_cli();
    let mut state = AppState::new(true, false);
    state.codex_home_override = Some(codex_home);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    let mut start_after_initialize = Some(StartMode {
        thread_action: StartupThreadAction::ResumePicker,
        initial_prompt: None,
    });

    handle_initialize_success(
        &cli,
        "/tmp/project",
        &mut state,
        &mut output,
        &mut writer,
        &mut start_after_initialize,
    )
    .expect("initialize success");

    assert!(state.startup_resume_picker);
    assert_eq!(state.last_listed_thread_ids, vec!["thr-new", "thr-old"]);
    assert!(state.pending.values().any(|pending| {
        matches!(
            pending,
            PendingRequest::ListThreads {
                search_term: None,
                cwd_filter: Some(cwd),
                source_kinds: None,
                view: ThreadListView::Threads,
            } if cwd == "/tmp/project"
        )
    }));
}

#[test]
fn thread_search_does_not_replace_ids_from_recent_thread_cache() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_home = temp.path().join("codex-home");
    persist_recent_thread_snapshot(Some(&codex_home), &cache_snapshot()).expect("persist cache");

    let cli = test_cli();
    let mut state = AppState::new(true, false);
    state.codex_home_override = Some(codex_home);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let handled = try_handle_thread_session_navigation(
        "threads",
        &["auth"],
        &cli,
        "/tmp/project",
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("handle threads")
    .expect("handled");

    assert!(handled);
    assert!(state.last_listed_thread_ids.is_empty());
}
