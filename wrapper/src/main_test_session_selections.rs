use std::process::Child;
use std::process::Command;
use std::process::Stdio;

use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::dispatch_command_session_meta::INIT_PROMPT;
use crate::dispatch_command_session_meta::current_rollout_message;
use crate::dispatch_submit_commands::try_handle_prefixed_submission;
use crate::editor::LineEditor;
use crate::events::process_server_line;
use crate::model_catalog::extract_models;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::requests::ThreadListView;
use crate::requests::send_windows_sandbox_setup_start;
use crate::state::AppState;
use crate::state::PendingSelection;

fn build_cli() -> Cli {
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
        prompt: Vec::new(),
    })
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

fn spawn_recording_stdin() -> (
    tempfile::TempDir,
    Child,
    std::process::ChildStdin,
    std::path::PathBuf,
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("requests.jsonl");
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("cat > \"$1\"")
        .arg("sh")
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn recorder");
    let stdin = child.stdin.take().expect("stdin");
    (temp, child, stdin, path)
}

fn read_recorded_requests(
    child: &mut Child,
    writer: std::process::ChildStdin,
    path: &std::path::Path,
) -> Vec<Value> {
    drop(writer);
    child.wait().expect("wait recorder");
    let contents = std::fs::read_to_string(path).expect("read requests");
    contents
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("parse request"))
        .collect()
}

fn test_codex_home() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_home = temp.path().join("codex-home");
    (temp, codex_home)
}

fn config_contents(codex_home: &std::path::Path) -> String {
    std::fs::read_to_string(codex_home.join("config.toml")).expect("read config")
}

#[test]
fn model_picker_applies_selected_model_and_effort() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    state.models = extract_models(&json!({
        "data": [{
            "id": "gpt-5-codex",
            "displayName": "GPT-5 Codex",
            "description": "Flagship coding model",
            "supportsPersonality": true,
            "isDefault": true,
            "defaultReasoningLevel": "medium",
            "supportedReasoningLevels": [
                {"effort": "low", "description": "fast"},
                {"effort": "medium", "description": "balanced"},
                {"effort": "high", "description": "deep"}
            ]
        }]
    }));
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/model",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("open model picker"),
        Some(true)
    );
    assert_eq!(state.pending_selection, Some(PendingSelection::Model));

    assert_eq!(
        try_handle_prefixed_submission(
            "1",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("select model"),
        Some(true)
    );
    assert_eq!(
        state.pending_selection,
        Some(PendingSelection::ReasoningEffort {
            model_id: "gpt-5-codex".to_string(),
        })
    );

    assert_eq!(
        try_handle_prefixed_submission(
            "3",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("select reasoning effort"),
        Some(true)
    );
    assert_eq!(
        state.session_overrides.model,
        Some(Some("gpt-5-codex".to_string()))
    );
    assert_eq!(
        state.session_overrides.reasoning_effort,
        Some(Some("high".to_string()))
    );
    assert_eq!(state.pending_selection, None);

    let contents = config_contents(&codex_home);
    assert!(contents.contains("model = \"gpt-5-codex\""));
    assert!(contents.contains("model_reasoning_effort = \"high\""));
}

#[test]
fn permissions_picker_updates_approval_and_sandbox_overrides() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/permissions",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("open permissions picker"),
        Some(true)
    );
    assert_eq!(state.pending_selection, Some(PendingSelection::Permissions));

    assert_eq!(
        try_handle_prefixed_submission(
            "2",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("select permissions preset"),
        Some(true)
    );
    assert_eq!(
        state.session_overrides.approval_policy.as_deref(),
        Some("on-request")
    );
    assert_eq!(
        state.session_overrides.thread_sandbox_mode.as_deref(),
        Some("workspace-write")
    );
    assert_eq!(state.pending_selection, None);
}

#[test]
fn fast_command_toggles_service_tier_override() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/fast",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("enable fast"),
        Some(true)
    );
    assert_eq!(
        state.session_overrides.service_tier,
        Some(Some("fast".to_string()))
    );
    assert!(config_contents(&codex_home).contains("service_tier = \"fast\""));

    assert_eq!(
        try_handle_prefixed_submission(
            "/fast",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("disable fast"),
        Some(true)
    );
    assert_eq!(state.session_overrides.service_tier, Some(None));
    assert!(!config_contents(&codex_home).contains("service_tier = "));
}

#[test]
fn personality_command_persists_selected_personality() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    state.models = extract_models(&json!({
        "data": [{
            "id": "gpt-5-codex",
            "displayName": "GPT-5 Codex",
            "supportsPersonality": true,
            "isDefault": true
        }]
    }));
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/personality friendly",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("set personality"),
        Some(true)
    );

    assert_eq!(
        state.session_overrides.personality,
        Some(Some("friendly".to_string()))
    );
    assert_eq!(state.active_personality.as_deref(), Some("friendly"));
    assert!(config_contents(&codex_home).contains("personality = \"friendly\""));
}

#[test]
fn theme_command_persists_selected_theme() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/theme base16-ocean.dark",
            &cli,
            "/tmp",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("set theme"),
        Some(true)
    );

    let contents = config_contents(&codex_home);
    assert!(contents.contains("[tui]"));
    assert!(contents.contains("theme = \"base16-ocean.dark\""));
}

#[test]
fn init_command_starts_new_thread_with_upstream_prompt() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let workspace = tempfile::tempdir().expect("tempdir");
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/init",
            &cli,
            workspace.path().to_str().expect("workspace path"),
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run init"),
        Some(true)
    );

    let pending = state.pending.values().next().expect("pending request");
    match pending {
        PendingRequest::StartThread { initial_prompt } => {
            assert_eq!(initial_prompt.as_deref(), Some(INIT_PROMPT.trim_end()));
        }
        other => panic!("expected StartThread, got {other:?}"),
    }
}

#[test]
fn init_command_uses_turn_start_when_thread_exists() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    let workspace = tempfile::tempdir().expect("tempdir");
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/init",
            &cli,
            workspace.path().to_str().expect("workspace path"),
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run init"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.first().expect("turn/start request");
    assert_eq!(request["method"], json!("turn/start"));
    assert_eq!(request["params"]["threadId"], json!("thread-1"));
    assert_eq!(request["params"]["input"][0]["type"], json!("text"));
    assert_eq!(
        request["params"]["input"][0]["text"],
        json!(INIT_PROMPT.trim_end())
    );
}

#[test]
fn init_command_skips_existing_agents_file() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("AGENTS.md"), "existing").expect("write AGENTS");
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/init",
            &cli,
            workspace.path().to_str().expect("workspace path"),
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run init"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    assert!(requests.is_empty());
    assert!(state.pending.is_empty());
}

#[test]
fn rollout_message_uses_current_path_when_available() {
    let mut state = AppState::new(true, false);
    state.current_rollout_path = Some(std::path::PathBuf::from("/tmp/codex-rollout.jsonl"));
    assert_eq!(
        current_rollout_message(&state),
        "Current rollout path: /tmp/codex-rollout.jsonl"
    );
}

#[test]
fn rollout_message_explains_missing_path() {
    let state = AppState::new(true, false);
    assert_eq!(
        current_rollout_message(&state),
        "Rollout path is not available yet."
    );
}

#[test]
fn agent_command_requests_filtered_agent_threads() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/agent",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run agent command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/list");
    assert_eq!(request["params"]["cwd"], "/tmp/project");
    assert_eq!(
        request["params"]["sourceKinds"],
        json!(["subAgentThreadSpawn"])
    );
}

#[test]
fn multi_agents_command_tracks_agent_thread_view() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/multi-agents",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run multi-agents command"),
        Some(true)
    );

    let pending = state.pending.values().next().expect("pending request");
    match pending {
        PendingRequest::ListThreads {
            source_kinds, view, ..
        } => {
            assert_eq!(source_kinds, &Some(vec!["subAgentThreadSpawn".to_string()]));
            assert_eq!(view, &ThreadListView::Agents);
        }
        other => panic!("expected agent thread list request, got {other:?}"),
    }
}

#[test]
fn new_thread_requests_advertise_client_dynamic_tools() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/new",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run new command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/start");
    let names = request["params"]["dynamicTools"]
        .as_array()
        .expect("dynamic tools")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "workspace_list_dir",
            "workspace_stat_path",
            "workspace_read_file",
            "workspace_find_files",
            "workspace_search_text",
            "background_shell_start",
            "background_shell_poll",
            "background_shell_list",
            "background_shell_terminate"
        ]
    );
}

#[test]
fn new_thread_omits_dynamic_tools_when_experimental_api_is_disabled() {
    let mut cli = build_cli();
    cli.no_experimental_api = true;
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/new",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run new command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/start");
    assert!(request["params"].get("dynamicTools").is_none());
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn new_thread_requests_persist_extended_history() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/new",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run new command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/start");
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn resume_requests_persist_extended_history() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/resume thread-77",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run resume command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/resume");
    assert_eq!(request["params"]["threadId"], "thread-77");
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn fork_requests_persist_extended_history() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-77".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/fork",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run fork command"),
        Some(true)
    );

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "thread/fork");
    assert_eq!(request["params"]["threadId"], "thread-77");
    assert_eq!(request["params"]["persistExtendedHistory"], true);
}

#[test]
fn windows_sandbox_setup_request_targets_workspace() {
    let mut state = AppState::new(true, false);
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();

    send_windows_sandbox_setup_start(&mut writer, &mut state, "/tmp/project", "elevated")
        .expect("send setup request");

    let requests = read_recorded_requests(&mut child, writer, &path);
    let request = requests.last().expect("request");
    assert_eq!(request["method"], "windowsSandbox/setupStart");
    assert_eq!(request["params"]["mode"], "elevated");
    assert_eq!(request["params"]["cwd"], "/tmp/project");
}

#[test]
fn setup_default_sandbox_is_scoped_to_windows() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let mut editor = LineEditor::default();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    assert_eq!(
        try_handle_prefixed_submission(
            "/setup-default-sandbox",
            &cli,
            "/tmp/project",
            &mut state,
            &mut editor,
            &mut output,
            &mut writer,
        )
        .expect("run setup-default-sandbox"),
        Some(true)
    );

    if cfg!(target_os = "windows") {
        let pending = state.pending.values().next().expect("pending request");
        match pending {
            PendingRequest::WindowsSandboxSetupStart { mode } => {
                assert_eq!(mode, "elevated");
            }
            other => panic!("expected windows sandbox setup request, got {other:?}"),
        }
    } else {
        assert!(state.pending.is_empty());
    }
}

#[test]
fn windows_sandbox_setup_completed_persists_mode() {
    let cli = build_cli();
    let mut state = AppState::new(true, false);
    let (_temp, codex_home) = test_codex_home();
    state.codex_home_override = Some(codex_home.clone());
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    let mut start_after_initialize = None;

    process_server_line(
        serde_json::to_string(&json!({
            "method": "windowsSandbox/setupCompleted",
            "params": {
                "mode": "elevated",
                "success": true,
                "error": null
            }
        }))
        .expect("serialize notification"),
        &cli,
        "/tmp/project",
        &mut state,
        &mut output,
        &mut writer,
        &mut start_after_initialize,
    )
    .expect("process notification");

    let contents = config_contents(&codex_home);
    assert!(contents.contains("[windows]"));
    assert!(contents.contains("sandbox = \"elevated\""));
}
