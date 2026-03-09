use super::super::super::*;

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
