use super::super::super::super::super::*;

#[test]
fn background_shell_inspect_capability_returns_provider_and_consumer_metadata() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
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
            }
        }),
        "/tmp",
        &manager,
    );
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "integration test",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let inspect_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_inspect_capability",
            "arguments": {
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(inspect_result["success"], true);
    let rendered = inspect_result["contentItems"][0]["text"]
        .as_str()
        .expect("inspect text");
    assert!(rendered.contains("Service capability: @api.http"));
    assert!(rendered.contains("bg-1 (dev api)  [untracked]"));
    assert!(rendered.contains("protocol http"));
    assert!(rendered.contains("endpoint http://127.0.0.1:4000"));
    assert!(rendered.contains("recipes  1"));
    assert!(rendered.contains("bg-2 (integration test)  [satisfied]  blocking=yes"));
    let _ = manager.terminate_all_running();
}
