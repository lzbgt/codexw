use super::super::super::*;

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
