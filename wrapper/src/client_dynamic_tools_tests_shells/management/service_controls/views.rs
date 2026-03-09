use super::super::*;

#[test]
fn background_shell_list_services_can_filter_by_capability() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
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
                "intent": "service",
                "label": "frontend svc",
                "capabilities": ["frontend.dev"]
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("service list text");
    assert!(text.contains("api svc"));
    assert!(text.contains("api.http"));
    assert!(!text.contains("frontend svc"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_list_services_rejects_invalid_capability() {
    let manager = BackgroundShellManager::default();
    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "capability": "@bad!"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error text")
            .contains("background shell capability")
    );
}

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

#[test]
fn background_shell_list_capabilities_can_filter_issue_classes() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let inspect_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_capabilities",
            "arguments": {
                "status": "missing"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(inspect_result["success"], true);
    let rendered = inspect_result["contentItems"][0]["text"]
        .as_str()
        .expect("list text");
    assert!(rendered.contains("@api.http -> <missing provider> [missing]"));
    assert!(rendered.contains("used by bg-1 [missing]"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_list_capabilities_can_filter_untracked_issue_class() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let inspect_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_capabilities",
            "arguments": {
                "status": "untracked"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(inspect_result["success"], true);
    let rendered = inspect_result["contentItems"][0]["text"]
        .as_str()
        .expect("list text");
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_list_capabilities_accepts_missing_arguments_object() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let inspect_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_capabilities"
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(inspect_result["success"], true);
    let rendered = inspect_result["contentItems"][0]["text"]
        .as_str()
        .expect("list text");
    assert!(rendered.contains("@api.http -> bg-1"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_list_services_can_filter_service_states() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) {
                    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "booting svc",
                "capabilities": ["svc.booting"],
                "readyPattern": "READY"
            }
        }),
        "/tmp",
        &manager,
    );
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) {
                    "echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "ready svc",
                "capabilities": ["svc.ready"],
                "readyPattern": "READY"
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
                "intent": "service",
                "label": "untracked svc",
                "capabilities": ["svc.untracked"]
            }
        }),
        "/tmp",
        &manager,
    );

    let wait_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_wait_ready",
            "arguments": {
                "jobId": "bg-2",
                "timeoutMs": 2_000
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(wait_result["success"], true);

    let ready_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "status": "ready"
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(ready_result["success"], true);
    let ready_text = ready_result["contentItems"][0]["text"]
        .as_str()
        .expect("ready text");
    assert!(ready_text.contains("ready svc"));
    assert!(!ready_text.contains("booting svc"));

    let booting_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "status": "booting"
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(booting_result["success"], true);
    let booting_text = booting_result["contentItems"][0]["text"]
        .as_str()
        .expect("booting text");
    assert!(booting_text.contains("booting svc"));
    assert!(!booting_text.contains("ready svc"));

    let untracked_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "status": "untracked"
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(untracked_result["success"], true);
    let untracked_text = untracked_result["contentItems"][0]["text"]
        .as_str()
        .expect("untracked text");
    assert!(untracked_text.contains("untracked svc"));
    assert!(!untracked_text.contains("ready svc"));
    let _ = manager.terminate_all_running();
}
