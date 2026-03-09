use super::*;

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
fn background_shell_update_service_can_reassign_capabilities() {
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

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_service",
            "arguments": {
                "jobId": "bg-1",
                "capabilities": ["frontend.dev"]
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Updated service metadata"));
    assert!(text.contains("reusable capabilities=@frontend.dev"));

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(!rendered.contains("@api.http"));
    assert!(rendered.contains("@frontend.dev"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_service_can_update_label() {
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

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_service",
            "arguments": {
                "jobId": "bg-1",
                "label": "frontend dev"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Updated service metadata"));
    assert!(text.contains("label=frontend dev"));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated service");
    assert!(rendered.contains("Label: frontend dev"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_service_can_update_contract_metadata() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "use /health"
            }
        }),
        "/tmp",
        &manager,
    );
    manager
        .send_input_for_operator("bg-1", "READY", true)
        .expect("send ready line");
    for _ in 0..40 {
        let rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll service");
        if rendered.contains("READY") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_service",
            "arguments": {
                "jobId": "bg-1",
                "protocol": "grpc",
                "endpoint": "grpc://127.0.0.1:50051",
                "attachHint": null,
                "readyPattern": "READY",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
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

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Updated service metadata"));
    assert!(text.contains("protocol=grpc"));
    assert!(text.contains("endpoint=grpc://127.0.0.1:50051"));
    assert!(text.contains("cleared attachHint"));
    assert!(text.contains("readyPattern=READY"));
    assert!(text.contains("recipes=1"));

    let rendered = manager.attach_for_operator("bg-1").expect("attach summary");
    assert!(rendered.contains("Protocol: grpc"));
    assert!(rendered.contains("Endpoint: grpc://127.0.0.1:50051"));
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("State: ready"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    assert!(!rendered.contains("Attach hint: use /health"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_service_can_clear_capabilities_with_null() {
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

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_service",
            "arguments": {
                "jobId": "bg-1",
                "capabilities": null
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("cleared reusable capabilities"));

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .map(|lines| lines.join("\n"))
        .unwrap_or_default();
    assert!(!rendered.contains("@api.http"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_dependencies_can_retarget_running_job() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_dependencies",
            "arguments": {
                "jobId": "bg-1",
                "dependsOnCapabilities": ["db.redis"]
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Updated dependency capabilities"));
    assert!(text.contains("@db.redis"));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated blocker");
    assert!(rendered.contains("Depends on capabilities: @db.redis"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_dependencies_can_clear_with_null() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_dependencies",
            "arguments": {
                "jobId": "bg-1",
                "dependsOnCapabilities": null
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Cleared dependency capabilities"));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated blocker");
    assert!(!rendered.contains("Depends on capabilities:"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_service_reports_field_specific_validation_errors() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service"
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_service",
            "arguments": {
                "jobId": "bg-1",
                "protocol": 123
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], false);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("error text");
    assert!(text.contains("`protocol` must be a string or null"));
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
