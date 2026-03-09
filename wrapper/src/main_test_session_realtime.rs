use crate::Cli;
use crate::dispatch_command_session_realtime::handle_realtime_command;
use crate::events::handle_realtime_notification;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::response_error_runtime::handle_runtime_error;
use crate::rpc::RpcNotification;
use crate::state::AppState;
use serde_json::json;
use std::process::Command;
use std::process::Stdio;
use std::time::Instant;

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
fn realtime_command_without_args_shows_status() {
    let cli = test_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result =
        handle_realtime_command(&[], &cli, &mut state, &mut output, &mut writer).expect("status");

    assert_eq!(result, Some(true));
}

#[test]
fn realtime_closed_clears_transient_prompt_and_session() {
    let cli = test_cli();
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.realtime_active = true;
    state.realtime_session_id = Some("rt-1".to_string());
    state.realtime_started_at = Some(Instant::now());
    state.realtime_prompt = Some("hello world".to_string());
    state.realtime_last_error = Some("last error".to_string());
    let mut output = Output::default();

    let handled = handle_realtime_notification(
        &RpcNotification {
            method: "thread/realtime/closed".to_string(),
            params: json!({ "reason": "done" }),
        },
        &cli,
        &mut state,
        &mut output,
    )
    .expect("closed");

    assert!(handled);
    assert!(!state.realtime_active);
    assert!(state.realtime_session_id.is_none());
    assert!(state.realtime_started_at.is_none());
    assert!(state.realtime_prompt.is_none());
    assert_eq!(state.realtime_last_error.as_deref(), Some("last error"));
}

#[test]
fn realtime_start_failure_clears_staged_prompt() {
    let mut state = AppState::new(true, false);
    state.realtime_prompt = Some("staged".to_string());
    let mut output = Output::default();

    let handled = handle_runtime_error(
        &json!({ "message": "boom" }),
        &PendingRequest::StartRealtime {
            prompt: "staged".to_string(),
        },
        &mut state,
        &mut output,
    )
    .expect("runtime error");

    assert!(handled);
    assert!(!state.realtime_active);
    assert!(state.realtime_session_id.is_none());
    assert!(state.realtime_started_at.is_none());
    assert!(state.realtime_prompt.is_none());
}
