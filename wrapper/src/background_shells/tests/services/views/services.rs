use super::super::super::*;
use crate::background_shells::BackgroundShellServiceIssueClass;

#[test]
fn service_shell_views_can_filter_ready_booting_untracked_and_conflicting_jobs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": delayed_service_ready_command(),
                "intent": "service",
                "label": "booting svc",
                "capabilities": ["svc.booting"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");
    manager
        .start_from_tool(
            &json!({
                "command": service_ready_command(),
                "intent": "service",
                "label": "ready svc",
                "capabilities": ["svc.ready"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready service");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "untracked svc",
                "capabilities": ["svc.untracked"]
            }),
            "/tmp",
        )
        .expect("start untracked service");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "conflict a",
                "capabilities": ["svc.conflict"]
            }),
            "/tmp",
        )
        .expect("start first conflicting service");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "conflict b",
                "capabilities": ["svc.conflict"]
            }),
            "/tmp",
        )
        .expect("start second conflicting service");

    manager
        .wait_ready_for_operator("bg-2", 2_000)
        .expect("wait for ready service");

    let ready = manager
        .render_service_shells_for_ps_filtered(Some(BackgroundShellServiceIssueClass::Ready), None)
        .expect("ready service render")
        .join("\n");
    assert!(ready.contains("ready svc"));
    assert!(!ready.contains("booting svc"));

    let booting = manager
        .render_service_shells_for_ps_filtered(
            Some(BackgroundShellServiceIssueClass::Booting),
            None,
        )
        .expect("booting service render")
        .join("\n");
    assert!(booting.contains("booting svc"));
    assert!(!booting.contains("ready svc"));

    let untracked = manager
        .render_service_shells_for_ps_filtered(
            Some(BackgroundShellServiceIssueClass::Untracked),
            None,
        )
        .expect("untracked service render")
        .join("\n");
    assert!(untracked.contains("untracked svc"));
    assert!(!untracked.contains("booting svc"));

    let conflicts = manager
        .render_service_shells_for_ps_filtered(
            Some(BackgroundShellServiceIssueClass::Conflicts),
            None,
        )
        .expect("conflict render")
        .join("\n");
    assert!(conflicts.contains("conflict a"));
    assert!(conflicts.contains("conflict b"));
    assert!(conflicts.contains("Capability conflicts:"));
    let _ = manager.terminate_all_running();
}
