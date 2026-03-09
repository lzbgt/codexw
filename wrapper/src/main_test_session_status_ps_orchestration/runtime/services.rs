use super::*;

#[test]
fn service_capability_reference_can_drive_ps_attach() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "attachHint": "Send HTTP requests to /health",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
                        "example": "curl http://127.0.0.1:4000/health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let attach_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "attach @api.http")
            .expect("attach");
    assert!(attach_result.contains("Service job: bg-1"));
    assert!(attach_result.contains("Capabilities: api.http"));
    assert!(attach_result.contains("health [http GET /health]: Check health"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_service_capability_index() {
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
        .expect("start service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered =
        handle_ps_command(&mut Output::plain_text(), &mut state, "capabilities").expect("caps");
    assert!(rendered.contains("Service capability index:"));
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    assert!(rendered.contains("used by bg-2 [satisfied]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_single_service_capability_detail() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health"
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "integration test",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "capabilities @api.http",
    )
    .expect("capability detail");
    assert!(rendered.contains("Service capability: @api.http"));
    assert!(rendered.contains("bg-1 (dev api)  [untracked]"));
    assert!(rendered.contains("protocol http"));
    assert!(rendered.contains("endpoint http://127.0.0.1:4000"));
    assert!(rendered.contains("recipes  1"));
    assert!(rendered.contains("bg-2 (integration test)  [satisfied]  blocking=yes"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_capability_index_by_issue_class() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "capabilities missing",
    )
    .expect("missing capabilities");
    assert!(rendered.contains("@api.http -> <missing provider> [missing]"));
    assert!(rendered.contains("used by bg-1 [missing]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_capability_index_by_untracked_issue_class() {
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
        .expect("start provider");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "capabilities untracked",
    )
    .expect("untracked capabilities");
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_dependencies_by_capability() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["db.redis"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start unrelated service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api dependency");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db dependency");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "dependencies missing @api.http",
    )
    .expect("focused dependency view");
    assert!(rendered.contains("Dependencies (@api.http):"));
    assert!(rendered.contains("shell:bg-2 -> capability:@api.http"));
    assert!(!rendered.contains("db.redis"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_service_shells_by_state() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) {
                    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "booting svc",
                "capabilities": ["svc.booting"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) {
                    "echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "ready svc",
                "capabilities": ["svc.ready"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "untracked svc",
                "capabilities": ["svc.untracked"]
            }),
            "/tmp",
        )
        .expect("start untracked service");

    handle_ps_command(&mut Output::plain_text(), &mut state, "wait bg-2 2000").expect("wait ready");

    let ready =
        handle_ps_command(&mut Output::plain_text(), &mut state, "services ready").expect("ready");
    assert!(ready.contains("ready svc"));
    assert!(!ready.contains("booting svc"));

    let booting = handle_ps_command(&mut Output::plain_text(), &mut state, "services booting")
        .expect("booting");
    assert!(booting.contains("booting svc"));
    assert!(!booting.contains("ready svc"));

    let untracked = handle_ps_command(&mut Output::plain_text(), &mut state, "services untracked")
        .expect("untracked");
    assert!(untracked.contains("untracked svc"));
    assert!(!untracked.contains("ready svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_service_shells_by_capability() {
    let mut state = crate::state::AppState::new(true, false);
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
        .expect("start api service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "frontend svc",
                "capabilities": ["frontend.dev"]
            }),
            "/tmp",
        )
        .expect("start frontend service");

    let rendered = handle_ps_command(&mut Output::plain_text(), &mut state, "services @api.http")
        .expect("service capability filter");
    assert!(rendered.contains("api svc"));
    assert!(rendered.contains("api.http"));
    assert!(!rendered.contains("frontend svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_conflicting_service_shells() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "conflict a",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first conflict");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "conflict b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second conflict");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "ok svc",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start non-conflict");

    let rendered = handle_ps_command(&mut Output::plain_text(), &mut state, "services conflicts")
        .expect("conflict filter");
    assert!(rendered.contains("conflict a"));
    assert!(rendered.contains("conflict b"));
    assert!(rendered.contains("Capability conflicts:"));
    assert!(!rendered.contains("ok svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_service_attachment_metadata_for_aliased_job() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "attachHint": "Send HTTP requests to /health",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check service health",
                        "example": "curl http://127.0.0.1:4000/health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let rendered =
        handle_ps_command(&mut Output::plain_text(), &mut state, "attach dev.api").expect("attach");
    assert!(rendered.contains("Service job: bg-1"));
    assert!(rendered.contains("Label: dev api"));
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:4000"));
    assert!(rendered.contains("Attach hint: Send HTTP requests to /health"));
    assert!(rendered.contains("health [http GET /health]: Check service health"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_wait_for_service_readiness_by_alias() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) {
                    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "dev api",
                "capabilities": ["api.http"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start service shell");
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let rendered = handle_ps_command(&mut Output::plain_text(), &mut state, "wait dev.api 2000")
        .expect("wait ready");
    assert!(rendered.contains("Service background shell job dev.api"));
    assert!(rendered.contains("ready"));
    assert!(rendered.contains("Ready pattern: READY"));
    let _ = state.background_shells.terminate_all_running();
}
