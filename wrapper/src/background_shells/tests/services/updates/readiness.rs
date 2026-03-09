use super::super::*;
use crate::background_shells::BackgroundShellServiceIssueClass;

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
