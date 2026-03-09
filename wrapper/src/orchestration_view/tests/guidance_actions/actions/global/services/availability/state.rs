use super::super::super::super::super::*;

#[test]
fn actions_filter_renders_contract_suggestions_for_untracked_services() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start untracked service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps services untracked"));
    assert!(rendered.contains(":ps contract bg-1 <json-object>"));
    assert!(rendered.contains(":ps relabel bg-1 <label|none>"));

    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_wait_for_single_booting_service() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps services booting"));
    assert!(rendered.contains(":ps wait bg-1 [timeoutMs]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn global_ready_service_actions_use_explicit_reference_syntax_when_provider_is_not_unique() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
                "readyPattern": "READY",
                "recipes": [{
                    "name": "health",
                    "action": {
                        "type": "stdin",
                        "text": "health"
                    }
                }]
            }),
            "/tmp",
        )
        .expect("start first ready service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
                "readyPattern": "READY",
                "recipes": [{
                    "name": "health",
                    "action": {
                        "type": "stdin",
                        "text": "health"
                    }
                }]
            }),
            "/tmp",
        )
        .expect("start second ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait ready service one");
    services
        .background_shells
        .wait_ready_for_operator("bg-2", 1000)
        .expect("wait ready service two");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps attach <jobId|alias|@capability|n>"));
    assert!(rendered.contains(":ps run <jobId|alias|@capability|n> <recipe> [json-args]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn global_booting_service_actions_use_explicit_reference_syntax_when_provider_is_not_unique() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start first booting service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start second booting service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps wait <jobId|alias|@capability|n> [timeoutMs]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn global_untracked_service_actions_use_explicit_reference_syntax_when_provider_is_not_unique() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start first untracked service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start second untracked service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps contract <jobId|alias|@capability|n> <json-object>"));
    assert!(rendered.contains(":ps relabel <jobId|alias|@capability|n> <label|none>"));
    let _ = services.background_shells.terminate_all_running();
}
