use super::super::*;

#[test]
fn completed_command_execution_clears_matching_running_status() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line =
        Some("running find frontend/src/components -maxdepth 2 -type f | sort".to_string());
    state.command_output_buffers.insert(
        "cmd-1".to_string(),
        "frontend/src/components/A.tsx\n".to_string(),
    );
    let mut output = Output::default();

    render_item_completed(
        &cli,
        &serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "cmd-1",
                "command": "find frontend/src/components -maxdepth 2 -type f | sort",
                "status": "completed",
                "exitCode": 0
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("render completed command");

    assert!(state.last_status_line.is_none());
}

#[test]
fn completed_command_execution_keeps_newer_status_line() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line = Some("waiting on approval".to_string());
    let mut output = Output::default();

    render_item_completed(
        &cli,
        &serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "cmd-1",
                "command": "find frontend/src/components -maxdepth 2 -type f | sort",
                "status": "completed",
                "exitCode": 0,
                "aggregatedOutput": ""
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("render completed command");

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on approval")
    );
}

#[test]
fn active_thread_status_without_flags_clears_stale_detail() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line = Some("waiting on approval".to_string());
    let mut output = Output::default();

    handle_realtime_notification(
        &crate::rpc::RpcNotification {
            method: "thread/status/changed".to_string(),
            params: serde_json::json!({
                "status": {"type": "active", "activeFlags": []}
            }),
        },
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle thread status");

    assert!(state.last_status_line.is_none());
}

#[test]
fn resolved_server_request_clears_waiting_on_approval_status() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line = Some("waiting on approval".to_string());
    let mut output = Output::default();

    let handled = handle_status_update(
        "serverRequest/resolved",
        &serde_json::json!({
            "threadId": "thread-1",
            "requestId": "req-1"
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle resolved request");

    assert!(handled);
    assert!(state.last_status_line.is_none());
}

#[test]
fn background_terminal_tracking_survives_turn_reset_and_shows_recent_output() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();

    handle_status_update(
        "item/started",
        &serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "cmd-1",
                "command": "python worker.py"
            }
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("start command item");
    handle_buffer_update(
        "item/commandExecution/outputDelta",
        &serde_json::json!({
            "itemId": "cmd-1",
            "delta": "booting\\nready\\n"
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("buffer output");
    handle_buffer_update(
        "item/commandExecution/terminalInteraction",
        &serde_json::json!({
            "itemId": "cmd-1",
            "processId": "proc-1",
            "stdin": ""
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("track background terminal");

    state.reset_turn_stream_state();

    assert_eq!(background_terminal_count(&state), 1);
    let rendered = render_background_terminals(&state);
    assert!(rendered.contains("python worker.py"));
    assert!(rendered.contains("proc-1"));
    assert!(rendered.contains("ready"));
}

#[test]
fn ready_status_mentions_blocking_prereqs_services_and_terminals() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-main".to_string());
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite shell");
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service shell");
    state.background_terminals.insert(
        "proc-1".to_string(),
        crate::background_terminals::BackgroundTerminalSummary {
            item_id: "cmd-1".to_string(),
            process_id: "proc-1".to_string(),
            command_display: "python worker.py".to_string(),
            waiting: true,
            recent_inputs: Vec::new(),
            recent_output: vec!["ready".to_string()],
        },
    );

    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("blocked on 1 prerequisite shell"));
    assert!(rendered.contains("1 service untracked"));
    assert!(rendered.contains("1 terminal"));
    assert!(rendered.contains(":ps to view"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn background_task_rendering_includes_local_background_shell_jobs() {
    let state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start background shell");

    let rendered = render_background_terminals(&state);
    assert!(rendered.contains("Local background shell jobs:"));
    assert!(rendered.contains("bg-1"));
    assert!(rendered.contains("sleep 0.4"));
    let _ = state.background_shells.terminate_all_running();
}
