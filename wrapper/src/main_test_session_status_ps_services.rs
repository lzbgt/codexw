use super::*;

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

#[test]
fn ps_command_can_update_service_contract_metadata() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "use /health"
            }),
            "/tmp",
        )
        .expect("start service shell");
    state
        .background_shells
        .send_input_for_operator("bg-1", "READY", true)
        .expect("send ready line");
    for _ in 0..40 {
        let rendered = state
            .background_shells
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll service output");
        if rendered.contains("READY") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    handle_ps_command(
        r#"contract 1 {"protocol":"grpc","endpoint":"grpc://127.0.0.1:50051","attachHint":null,"readyPattern":"READY","recipes":[{"name":"health","description":"Check health","action":{"type":"http","method":"GET","path":"/health"}}]}"#,
        &[
            "contract",
            "1",
            r#"{"protocol":"grpc","endpoint":"grpc://127.0.0.1:50051","attachHint":null,"readyPattern":"READY","recipes":[{"name":"health","description":"Check health","action":{"type":"http","method":"GET","path":"/health"}}]}"#,
        ],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("update service contract");

    let rendered = state
        .background_shells
        .attach_for_operator("bg-1")
        .expect("attach updated service");
    assert!(rendered.contains("Protocol: grpc"));
    assert!(rendered.contains("Endpoint: grpc://127.0.0.1:50051"));
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("State: ready"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    assert!(!rendered.contains("Attach hint: use /health"));
    let _ = state.background_shells.terminate_all_running();
}
