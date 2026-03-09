use super::super::super::*;

#[test]
fn actions_filter_uses_concrete_wait_for_booting_blocker_provider() {
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
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start blocked prerequisite");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps services booting @api.http"));
    assert!(rendered.contains(":ps wait bg-1 5000"));
    assert!(rendered.contains(":ps dependencies booting @api.http"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
    assert!(tool_rendered.contains(
        "background_shell_list_services {\"status\":\"booting\",\"capability\":\"@api.http\"}"
    ));
    assert!(
        tool_rendered
            .contains("background_shell_wait_ready {\"jobId\":\"bg-1\",\"timeoutMs\":5000}")
    );
    assert!(tool_rendered.contains(
        "orchestration_list_dependencies {\"filter\":\"booting\",\"capability\":\"@api.http\"}"
    ));

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps wait bg-1 5000"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_poll_for_single_generic_blocker() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite"
            }),
            "/tmp",
        )
        .expect("start generic blocker");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps blockers"));
    assert!(rendered.contains(":ps poll bg-1"));
    assert!(rendered.contains(":clean blockers"));
    assert!(!rendered.contains(":ps wait <job> [timeoutMs]"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
    assert!(tool_rendered.contains("orchestration_list_workers {\"filter\":\"blockers\"}"));
    assert!(tool_rendered.contains("background_shell_poll {\"jobId\":\"bg-1\"}"));
    assert!(tool_rendered.contains("background_shell_clean {\"scope\":\"blockers\"}"));
    assert!(!tool_rendered.contains("background_shell_wait_ready"));

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps poll bg-1"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_real_reference_placeholder_for_non_unique_generic_blockers() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite"
            }),
            "/tmp",
        )
        .expect("start first generic blocker");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite"
            }),
            "/tmp",
        )
        .expect("start second generic blocker");

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(
        tool_rendered.contains("background_shell_poll {\"jobId\":\"<jobId|alias|@capability>\"}")
    );
    assert!(!tool_rendered.contains("background_shell_poll {\"jobId\":\"bg-...\"}"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_provider_ref_for_missing_capability_when_unique_service_exists() {
    let blocked = crate::state::AppState::new(true, false);
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start service");
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    let rendered = render_orchestration_actions(&blocked);
    assert!(rendered.contains(":ps provide bg-1 @api.http"));
    assert!(rendered.contains(":ps depend bg-2 <@capability...|none>"));

    let tool_rendered = render_orchestration_actions_for_tool(&blocked);
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@api.http\"]}"
    ));
    assert!(tool_rendered.contains(
        "background_shell_update_dependencies {\"jobId\":\"bg-2\",\"dependsOnCapabilities\":[\"@other.role\"]}"
    ));
    let _ = blocked.background_shells.terminate_all_running();
}
