use super::super::super::super::*;

#[test]
fn ps_command_can_relabel_service_shell() {
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
        "relabel 1 frontend dev",
        &["relabel", "1", "frontend", "dev"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("relabel service shell");

    let rendered = state
        .background_shells
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll relabeled service");
    assert!(rendered.contains("Label: frontend dev"));

    handle_ps_command(
        "relabel 1 none",
        &["relabel", "1", "none"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("clear service label");

    let rendered = state
        .background_shells
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll cleared label");
    assert!(!rendered.contains("Label: frontend dev"));
    let _ = state.background_shells.terminate_all_running();
}
