use super::super::*;

#[test]
fn background_shell_manager_resolves_job_references_by_id_and_index() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start shell 1");
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start shell 2");

    assert_eq!(
        manager
            .resolve_job_reference("bg-1")
            .expect("resolve by id"),
        "bg-1"
    );
    assert_eq!(
        manager
            .resolve_job_reference("2")
            .expect("resolve by index"),
        "bg-2"
    );
    manager.set_job_alias("bg-2", "dev.api").expect("set alias");
    assert_eq!(
        manager
            .resolve_job_reference("dev.api")
            .expect("resolve by alias"),
        "bg-2"
    );
    assert!(manager.resolve_job_reference("0").is_err());
    assert!(manager.resolve_job_reference("bg-9").is_err());
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_manager_can_set_and_clear_aliases() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service", "label": "dev server"}),
            "/tmp",
        )
        .expect("start shell");

    manager
        .set_job_alias("bg-1", "dev_server")
        .expect("set alias");
    let snapshots = manager.snapshots();
    assert_eq!(snapshots[0].alias.as_deref(), Some("dev_server"));
    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll background shell");
    assert!(rendered.contains("Alias: dev_server"));

    let cleared = manager.clear_job_alias("dev_server").expect("clear alias");
    assert_eq!(cleared, "bg-1");
    let snapshots = manager.snapshots();
    assert!(snapshots[0].alias.is_none());
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_send_from_tool_resolves_aliases() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": interactive_echo_command()}), "/tmp")
        .expect("start shell");
    manager.set_job_alias("bg-1", "dev.api").expect("set alias");

    let rendered = manager
        .send_input_from_tool(&json!({
            "jobId": "dev.api",
            "text": "ping via alias"
        }))
        .expect("send via alias");

    assert!(rendered.contains("Sent"));
    let _ = manager.terminate_all_running();
}
