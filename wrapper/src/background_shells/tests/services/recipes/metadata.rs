use super::super::*;

#[test]
fn service_attachment_summary_exposes_endpoint_and_attach_hint() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "attachHint": "Send HTTP requests to /health",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check service health",
                        "example": "curl http://127.0.0.1:4000/health",
                        "parameters": [
                            {
                                "name": "id",
                                "description": "Resource id",
                                "default": "health"
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/{{id}}"
                        }
                    },
                    {
                        "name": "metrics",
                        "description": "Fetch metrics",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/metrics"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .attach_for_operator("bg-1")
        .expect("render attachment summary");
    assert!(rendered.contains("Service job: bg-1"));
    assert!(rendered.contains("Label: dev api"));
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:4000"));
    assert!(rendered.contains("Attach hint: Send HTTP requests to /health"));
    assert!(rendered.contains("health [http GET /{{id}}]: Check service health"));
    assert!(rendered.contains("params: id=health"));
    assert!(rendered.contains("example: curl http://127.0.0.1:4000/health"));
    assert!(rendered.contains("metrics [http GET /metrics]: Fetch metrics"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_parameters_support_defaults_and_substitution() {
    let endpoint = spawn_test_http_server("GET", "/items/default-id", "ok");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "recipes": [
                    {
                        "name": "item",
                        "description": "Fetch one item",
                        "parameters": [
                            {
                                "name": "id",
                                "default": "default-id"
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/items/{{id}}"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator_with_args("bg-1", "item", &HashMap::new())
        .expect("invoke defaulted recipe");
    assert!(rendered.contains("Action: http GET /items/default-id"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_parameters_can_be_overridden() {
    let endpoint = spawn_test_http_server("GET", "/items/42", "ok");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "recipes": [
                    {
                        "name": "item",
                        "description": "Fetch one item",
                        "parameters": [
                            {
                                "name": "id",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/items/{{id}}"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator_with_args(
            "bg-1",
            "item",
            &HashMap::from([("id".to_string(), "42".to_string())]),
        )
        .expect("invoke overridden recipe");
    assert!(rendered.contains("Action: http GET /items/42"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_missing_required_parameter_fails() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "recipes": [
                    {
                        "name": "item",
                        "description": "Fetch one item",
                        "parameters": [
                            {
                                "name": "id",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/items/{{id}}"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "item")
        .expect_err("missing required parameter should fail");
    assert!(err.contains("parameter `id` is required"));
    let _ = manager.terminate_all_running();
}

#[test]
fn informational_recipe_cannot_be_invoked() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "recipes": [
                    {
                        "name": "docs",
                        "description": "Read the operator runbook"
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "docs")
        .expect_err("informational recipe should fail");
    assert!(err.contains("descriptive only"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_attachment_fields_require_service_intent() {
    let manager = BackgroundShellManager::default();

    let err = manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "observation",
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "attachHint": "Use /health"
            }),
            "/tmp",
        )
        .expect_err("non-service attachment metadata should fail");
    assert!(err.contains("`protocol`, `endpoint`, `attachHint`, `recipes`"));
}
