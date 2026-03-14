use super::super::super::*;

#[test]
fn ps_command_can_poll_and_terminate_specific_background_shell_jobs() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start first shell");
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start second shell");

    let poll = handle_ps_command(&mut Output::plain_text(), &mut state, "poll bg-1").expect("poll");
    assert!(poll.contains("Job: bg-1"));

    let terminate =
        handle_ps_command(&mut Output::plain_text(), &mut state, "terminate 2").expect("terminate");
    assert!(terminate.contains("Terminated background shell job bg-2"));

    let rendered = state
        .background_shells
        .render_for_ps_filtered(None)
        .expect("render shells")
        .join("\n");
    assert!(rendered.contains("bg-1"));
    assert!(rendered.contains("bg-2"));
    assert!(rendered.contains("terminated"));
    let _ = state.background_shells.terminate_all_running();
}
