use super::super::super::*;

#[test]
fn background_shell_start_preserves_request_origin_metadata() {
    let manager = BackgroundShellManager::default();
    let result = execute_dynamic_tool_call(
        &json!({
            "threadId": "thread-agent-1",
            "callId": "call-55",
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "repo build",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let snapshots = manager.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].origin.source_thread_id.as_deref(),
        Some("thread-agent-1")
    );
    assert_eq!(
        snapshots[0].origin.source_call_id.as_deref(),
        Some("call-55")
    );
    assert_eq!(snapshots[0].intent.as_str(), "prerequisite");
    assert_eq!(snapshots[0].label.as_deref(), Some("repo build"));
    assert_eq!(
        snapshots[0].dependency_capabilities,
        vec!["api.http".to_string()]
    );
    let _ = manager.terminate_all_running();
}
