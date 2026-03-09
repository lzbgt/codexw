use super::super::super::*;

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
