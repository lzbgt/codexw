use super::*;
use crate::background_shells::BackgroundShellServiceIssueClass;

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

#[test]
fn running_service_contract_can_be_updated_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
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
        .expect("start service");
    manager
        .send_input_for_operator("bg-1", "READY", true)
        .expect("send ready line");
    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll service output");
        if rendered.contains("READY") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(rendered.contains("READY"));

    manager
        .set_running_service_contract(
            "bg-1",
            Some(Some("grpc".to_string())),
            Some(Some("grpc://127.0.0.1:50051".to_string())),
            Some(None),
            Some(Some("READY".to_string())),
            Some(vec![
                crate::background_shells::BackgroundShellInteractionRecipe {
                    name: "health".to_string(),
                    description: Some("Check health".to_string()),
                    example: None,
                    parameters: Vec::new(),
                    action: crate::background_shells::BackgroundShellInteractionAction::Http {
                        method: "GET".to_string(),
                        path: "/health".to_string(),
                        body: None,
                        headers: Vec::new(),
                        expected_status: None,
                    },
                },
            ]),
        )
        .expect("update contract");

    let rendered = manager.attach_for_operator("bg-1").expect("attach summary");
    assert!(rendered.contains("Protocol: grpc"));
    assert!(rendered.contains("Endpoint: grpc://127.0.0.1:50051"));
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("State: ready"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    assert!(!rendered.contains("Attach hint: use /health"));
    let _ = manager.terminate_all_running();
}

#[test]
fn running_dependency_capabilities_can_be_updated_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
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

    let updated = manager
        .set_running_dependency_capabilities("bg-1", &["db.redis".to_string()])
        .expect("update dependency capabilities");
    assert_eq!(updated, vec!["db.redis".to_string()]);

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated blocker");
    assert!(rendered.contains("Depends on capabilities: @db.redis"));
    assert!(!rendered.contains("Depends on capabilities: @api.http"));
    let _ = manager.terminate_all_running();
}

#[test]
fn terminate_running_services_by_capability_terminates_all_matching_providers() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api a",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "db",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start unrelated provider");

    let terminated = manager
        .terminate_running_services_by_capability("api.http")
        .expect("terminate matching providers");
    assert_eq!(terminated, 2);

    let remaining = manager
        .render_service_shells_for_ps_filtered(None, None)
        .expect("render remaining services")
        .join("\n");
    assert!(!remaining.contains("api a"));
    assert!(!remaining.contains("api b"));
    assert!(remaining.contains("db"));
    let _ = manager.terminate_all_running();
}

#[test]
fn terminate_running_blockers_by_capability_terminates_only_matching_prereqs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api blocker");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "db blocker",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    let terminated = manager
        .terminate_running_blockers_by_capability("api.http")
        .expect("terminate matching blockers");
    assert_eq!(terminated, 1);

    let remaining = manager
        .capability_dependency_summaries()
        .into_iter()
        .map(|summary| format!("{} -> {}", summary.job_id, summary.capability))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!remaining.contains("api.http"));
    assert!(remaining.contains("db.redis"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_shell_ready_pattern_transitions_from_booting_to_ready() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": delayed_service_ready_command(),
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start service");

    let booting = manager
        .render_service_shells_for_ps_filtered(
            Some(BackgroundShellServiceIssueClass::Booting),
            None,
        )
        .expect("booting render")
        .join("\n");
    assert!(booting.contains("bg-1"));

    manager
        .wait_ready_for_operator("bg-1", 2_000)
        .expect("wait ready");

    let ready = manager
        .render_service_shells_for_ps_filtered(Some(BackgroundShellServiceIssueClass::Ready), None)
        .expect("ready render")
        .join("\n");
    assert!(ready.contains("bg-1"));
    let _ = manager.terminate_all_running();
}

#[test]
fn wait_ready_for_operator_reports_service_readiness() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": delayed_service_ready_command(),
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start service");

    let rendered = manager
        .wait_ready_for_operator("bg-1", 2_000)
        .expect("wait ready");
    assert!(rendered.contains("Service background shell job bg-1 became ready"));
    assert!(rendered.contains("Ready pattern: READY"));
    let _ = manager.terminate_all_running();
}

#[test]
fn wait_ready_for_operator_rejects_untracked_services() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start service");

    let err = manager
        .wait_ready_for_operator("bg-1", 500)
        .expect_err("untracked service should reject wait");
    assert!(err.contains("does not declare a `readyPattern`; readiness is untracked"));
    let _ = manager.terminate_all_running();
}

#[test]
fn ready_pattern_requires_service_intent() {
    let manager = BackgroundShellManager::default();
    let err = manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.1",
                "intent": "observation",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect_err("readyPattern should require service intent");
    assert!(err.contains("readyPattern"));
    assert_eq!(manager.job_count(), 0);
}
