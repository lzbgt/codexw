use super::*;

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
