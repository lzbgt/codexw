use super::super::super::super::super::*;

#[test]
fn service_capability_index_lists_running_service_roles() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http", "frontend.dev"]
            }),
            "/tmp",
        )
        .expect("start provider");

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    assert!(rendered.contains("@frontend.dev -> bg-1 [untracked]"));
    let _ = manager.terminate_all_running();
}

#[test]
fn capability_index_can_render_consumers_of_reusable_services() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    assert!(rendered.contains("used by bg-2 [satisfied]"));
    let _ = manager.terminate_all_running();
}

#[test]
fn capability_index_can_render_missing_providers_for_declared_dependencies() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(rendered.contains("@api.http -> <missing provider> [missing]"));
    assert!(rendered.contains("used by bg-1 [missing]"));
    let _ = manager.terminate_all_running();
}
