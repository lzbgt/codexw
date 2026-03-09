use super::super::super::*;

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
