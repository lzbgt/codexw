use super::super::super::super::super::*;

#[test]
fn background_shell_attach_returns_service_metadata() {
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
                "attachHint": "Send HTTP requests to /health",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
                        "example": "curl http://127.0.0.1:4000/health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );
    manager.set_job_alias("bg-1", "dev.api").expect("set alias");

    let attach_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_attach",
            "arguments": {
                "jobId": "@api.http"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(attach_result["success"], true);
    let rendered = attach_result["contentItems"][0]["text"]
        .as_str()
        .expect("attach text");
    assert!(rendered.contains("Service job: bg-1"));
    assert!(rendered.contains("Capabilities: api.http"));
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:4000"));
    assert!(rendered.contains("Attach hint: Send HTTP requests to /health"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    let _ = manager.terminate_all_running();
}
