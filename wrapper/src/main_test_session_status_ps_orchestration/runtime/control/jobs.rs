use super::*;

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
    assert!(rendered.contains("[terminated]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_alias_and_reuse_background_shell_job_references() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service shell");

    let alias_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
            .expect("alias");
    assert!(alias_result.contains("Aliased background shell job bg-1 as dev.api"));

    let poll_result = handle_ps_command(&mut Output::plain_text(), &mut state, "poll dev.api")
        .expect("poll alias");
    assert!(poll_result.contains("Job: bg-1"));
    assert!(poll_result.contains("Alias: dev.api"));

    let attach_result = handle_ps_command(&mut Output::plain_text(), &mut state, "attach dev.api")
        .expect("attach alias");
    assert!(attach_result.contains("Service job: bg-1"));

    let terminate_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "terminate dev.api")
            .expect("terminate alias");
    assert!(terminate_result.contains("Terminated background shell job bg-1"));
}

#[test]
fn ps_command_can_alias_and_unalias_background_shell_job_by_capability_reference() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let alias_result = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "alias @api.http dev.api",
    )
    .expect("alias capability");
    assert!(alias_result.contains("Aliased background shell job bg-1 as dev.api"));

    let unalias_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "unalias @api.http")
            .expect("unalias capability");
    assert!(unalias_result.contains("Cleared alias for background shell job bg-1"));
    assert!(
        state
            .background_shells
            .resolve_job_reference("dev.api")
            .is_err()
    );
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_send_input_to_aliased_background_shell_job() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start service shell");
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let send_result = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "send dev.api ping from ps",
    )
    .expect("send input");
    assert!(send_result.contains("Sent input to background shell job bg-1"));

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered =
            handle_ps_command(&mut Output::plain_text(), &mut state, "poll dev.api").expect("poll");
        if rendered.contains("ping from ps") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(rendered.contains("ping from ps"));
    let _ = state.background_shells.terminate_all_running();
}
