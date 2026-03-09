use super::super::super::*;

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
