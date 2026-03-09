use super::*;

#[test]
fn service_recipe_can_invoke_http_action() {
    let endpoint = spawn_test_http_server("GET", "/health", "ok");
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
                        "name": "health",
                        "description": "Check service health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "health")
        .expect("invoke http recipe");
    assert!(rendered.contains("Action: http GET /health"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    assert!(rendered.contains("ok"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_http_action_waits_for_booting_service_readiness() {
    let endpoint = spawn_test_http_server("GET", "/health", "ok");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": delayed_service_ready_command(),
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
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
            }),
            "/tmp",
        )
        .expect("start service shell");

    let started = std::time::Instant::now();
    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "health")
        .expect("invoke recipe after readiness wait");
    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(rendered.contains("Readiness: waited"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_can_invoke_http_action_with_headers_body_and_expected_status() {
    let endpoint = spawn_test_http_server_with_assertions(
        |request| {
            assert!(request.starts_with("POST /seed HTTP/1.1\r\n"));
            assert!(request.contains("Authorization: Bearer demo\r\n"));
            assert!(request.contains("Content-Type: application/x-www-form-urlencoded\r\n"));
            assert!(request.contains("\r\n\r\nseed=true"));
        },
        "HTTP/1.1 202 Accepted\r\nContent-Length: 6\r\nConnection: close\r\n\r\nseeded",
    );
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
                        "name": "seed",
                        "description": "Seed data",
                        "action": {
                            "type": "http",
                            "method": "POST",
                            "path": "/seed",
                            "body": "seed=true",
                            "headers": {
                                "Authorization": "Bearer demo",
                                "Content-Type": "application/x-www-form-urlencoded"
                            },
                            "expectedStatus": 202
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "seed")
        .expect("invoke rich http recipe");
    assert!(rendered.contains("Action: http POST /seed headers=2 body=9b expect=202"));
    assert!(rendered.contains("Status: HTTP/1.1 202 Accepted"));
    assert!(rendered.contains("Status code: 202"));
    assert!(rendered.contains("seeded"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_http_expected_status_is_enforced() {
    let endpoint = spawn_test_http_server("GET", "/health", "not-ready");
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
                        "name": "health",
                        "description": "Check service health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health",
                            "expectedStatus": 204
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "health")
        .expect_err("expected status mismatch should fail");
    assert!(err.contains("expected status 204"));
    assert!(err.contains("Status code: 200"));
    let _ = manager.terminate_all_running();
}
