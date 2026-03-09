use super::super::super::super::super::*;

#[test]
fn running_service_capabilities_can_be_reassigned_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service");

    let updated = manager
        .set_running_service_capabilities("bg-1", &["frontend.dev".to_string()])
        .expect("update capabilities");
    assert_eq!(updated, vec!["frontend.dev".to_string()]);

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(!rendered.contains("@api.http"));
    assert!(rendered.contains("@frontend.dev"));
    let _ = manager.terminate_all_running();
}

#[test]
fn running_service_label_can_be_updated_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service");

    let updated = manager
        .set_running_service_label("bg-1", Some("frontend dev".to_string()))
        .expect("update label");
    assert_eq!(updated, Some("frontend dev".to_string()));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated service");
    assert!(rendered.contains("Label: frontend dev"));

    let cleared = manager
        .set_running_service_label("bg-1", None)
        .expect("clear label");
    assert_eq!(cleared, None);

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll cleared label");
    assert!(!rendered.contains("Label: frontend dev"));
    let _ = manager.terminate_all_running();
}
