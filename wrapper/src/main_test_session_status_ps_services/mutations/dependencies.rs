use super::super::super::*;

#[test]
fn ps_command_can_retarget_dependency_capabilities() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start blocker");

    handle_ps_command(
        "depend 1 @db.redis",
        &["depend", "1", "@db.redis"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("retarget dependency capabilities");

    let rendered = state
        .background_shells
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll retargeted blocker");
    assert!(rendered.contains("Depends on capabilities: @db.redis"));
    assert!(!rendered.contains("Depends on capabilities: @api.http"));
    let _ = state.background_shells.terminate_all_running();
}
