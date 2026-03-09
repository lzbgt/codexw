use super::*;
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

#[test]
fn service_capability_reference_resolves_unique_service_job() {
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

    let resolved = manager
        .resolve_job_reference("@api.http")
        .expect("resolve unique capability");
    assert_eq!(resolved, "bg-1");
    let _ = manager.terminate_all_running();
}

#[test]
fn service_capability_reference_errors_when_ambiguous() {
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
        .expect("start first provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second provider");

    let err = manager
        .resolve_job_reference("@api.http")
        .expect_err("ambiguous capability should fail");
    assert!(err.contains("is ambiguous across multiple running service jobs"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_capability_reference_ignores_completed_service_jobs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.1",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start provider");
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(1) {
        let snapshots = manager.snapshots();
        if snapshots
            .iter()
            .all(|snapshot| snapshot.id != "bg-1" || snapshot.status != "running")
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let err = manager
        .resolve_job_reference("@api.http")
        .expect_err("completed provider should be ignored");
    assert!(err.contains("unknown running background shell capability `@api.http`"));
}

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

#[test]
fn single_capability_view_renders_providers_and_consumers() {
    let manager = BackgroundShellManager::default();
    manager
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
    manager
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

    let rendered = manager
        .render_single_service_capability_for_ps("api.http")
        .expect("capability detail")
        .join("\n");
    assert!(rendered.contains("Service capability: @api.http"));
    assert!(rendered.contains("bg-1 (dev api)  [untracked]"));
    assert!(rendered.contains("protocol http"));
    assert!(rendered.contains("endpoint http://127.0.0.1:4000"));
    assert!(rendered.contains("recipes  1"));
    assert!(rendered.contains("bg-2 (integration test)  [satisfied]  blocking=yes"));
    let _ = manager.terminate_all_running();
}
