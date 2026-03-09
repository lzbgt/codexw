use super::super::super::super::*;

#[test]
fn ps_command_can_reassign_service_capabilities() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service shell");

    handle_ps_command(
        "provide 1 @frontend.dev @frontend.hmr",
        &["provide", "1", "@frontend.dev", "@frontend.hmr"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("reassign service capabilities");

    let rendered = state
        .background_shells
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(!rendered.contains("@api.http"));
    assert!(rendered.contains("@frontend.dev"));
    assert!(rendered.contains("@frontend.hmr"));
    let _ = state.background_shells.terminate_all_running();
}
