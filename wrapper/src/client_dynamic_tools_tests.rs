use super::dynamic_tool_specs;
use super::execute_dynamic_tool_call;
use super::execute_dynamic_tool_call_with_state;
use crate::background_shells::BackgroundShellManager;
use crate::state::AppState;
use serde_json::json;

#[test]
fn dynamic_tool_specs_include_workspace_tools() {
    let specs = dynamic_tool_specs();
    let names = specs
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "orchestration_status",
            "orchestration_list_workers",
            "orchestration_list_dependencies",
            "workspace_list_dir",
            "workspace_stat_path",
            "workspace_read_file",
            "workspace_find_files",
            "workspace_search_text",
            "background_shell_start",
            "background_shell_poll",
            "background_shell_send",
            "background_shell_list_capabilities",
            "background_shell_list_services",
            "background_shell_inspect_capability",
            "background_shell_attach",
            "background_shell_wait_ready",
            "background_shell_invoke_recipe",
            "background_shell_list",
            "background_shell_terminate"
        ]
    );
}

#[test]
fn orchestration_status_reports_worker_and_guidance_summary() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_status"
        }),
        "/tmp",
        &state,
    );

    assert_eq!(result["success"], true);
    let rendered = result["contentItems"][0]["text"]
        .as_str()
        .expect("status text");
    assert!(rendered.contains("orchestration   main=1"));
    assert!(rendered.contains("cap_deps_missing=1"));
    assert!(rendered.contains("next action"));
    assert!(rendered.contains("missing service capability @api.http"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_list_workers_supports_filtered_capability_and_guidance_views() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    let caps = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "capabilities"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(caps["success"], true);
    let caps_text = caps["contentItems"][0]["text"]
        .as_str()
        .expect("capabilities text");
    assert!(caps_text.contains("Service capability index:"));
    assert!(caps_text.contains("@api.http -> <missing provider> [missing]"));

    let deps = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "dependencies"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(deps["success"], true);
    let deps_text = deps["contentItems"][0]["text"]
        .as_str()
        .expect("dependency text");
    assert!(deps_text.contains("Dependencies:"));
    assert!(!deps_text.contains("Main agent state:"));
    assert!(deps_text.contains("shell:bg-1 -> capability:@api.http"));

    let guidance = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "guidance"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(guidance["success"], true);
    let guidance_text = guidance["contentItems"][0]["text"]
        .as_str()
        .expect("guidance text");
    assert!(guidance_text.contains("missing service capability @api.http"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_list_dependencies_supports_issue_filters() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    let missing = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "missing"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(missing["success"], true);
    let missing_text = missing["contentItems"][0]["text"]
        .as_str()
        .expect("missing dependency text");
    assert!(missing_text.contains("Dependencies:"));
    assert!(
        missing_text.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        )
    );

    let sidecars = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "sidecars"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(sidecars["success"], true);
    let sidecar_text = sidecars["contentItems"][0]["text"]
        .as_str()
        .expect("sidecar dependency text");
    assert!(sidecar_text.contains("No sidecar dependency edges tracked right now."));

    let focused = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "missing",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(focused["success"], true);
    let focused_text = focused["contentItems"][0]["text"]
        .as_str()
        .expect("focused dependency text");
    assert!(focused_text.contains("Dependencies (@api.http):"));
    assert!(
        focused_text.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        )
    );
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_list_workers_rejects_unknown_filters() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "weird"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains("orchestration_list_workers `filter`")
    );
}

#[test]
fn orchestration_list_dependencies_rejects_unknown_filters() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "weird"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains("orchestration_list_dependencies `filter`")
    );
}

#[test]
fn orchestration_list_dependencies_rejects_empty_capability_argument() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "capability": "@"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains("orchestration_list_dependencies `capability`")
    );
}

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
fn workspace_list_dir_returns_sorted_entries() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(workspace.path().join("src")).expect("mkdir");
    std::fs::write(workspace.path().join("a.txt"), "alpha").expect("write");
    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn demo() {}\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_list_dir",
            "arguments": {"path": ".", "limit": 10}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("Directory: ."));
    assert!(text.contains("file  5 bytes"));
    assert!(text.contains("a.txt"));
    assert!(text.contains("dir   -"));
    assert!(text.contains("src"));
}

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

#[test]
fn background_shell_send_writes_to_alias_target() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "echo shell"
            }
        }),
        "/tmp",
        &manager,
    );
    manager.set_job_alias("bg-1", "dev.api").expect("set alias");

    let send_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_send",
            "arguments": {
                "jobId": "dev.api",
                "text": "ping from tool"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(send_result["success"], true);
    let mut rendered = String::new();
    for _ in 0..40 {
        let poll_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_poll",
                "arguments": {
                    "jobId": "dev.api"
                }
            }),
            "/tmp",
            &manager,
        );
        rendered = poll_result["contentItems"][0]["text"]
            .as_str()
            .expect("poll text")
            .to_string();
        if rendered.contains("ping from tool") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    assert!(rendered.contains("ping from tool"));
    let _ = manager.terminate_all_running();
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

#[test]
fn background_shell_invoke_recipe_runs_structured_service_action() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert!(request.starts_with("GET /health HTTP/1.1\r\n"));
        std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        )
        .expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.health"],
                "protocol": "http",
                "endpoint": format!("http://{addr}"),
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
    manager.set_job_alias("bg-1", "dev.api").expect("set alias");

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "@api.health",
                "recipe": "health"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: http GET /health"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_http_headers_body_and_expected_status() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert!(request.starts_with("POST /seed HTTP/1.1\r\n"));
        assert!(request.contains("Authorization: Bearer demo\r\n"));
        assert!(request.contains("\r\n\r\nseed=true"));
        std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 202 Accepted\r\nContent-Length: 7\r\nConnection: close\r\n\r\nseeded!",
        )
        .expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": format!("http://{addr}"),
                "recipes": [
                    {
                        "name": "seed",
                        "description": "Seed the service",
                        "action": {
                            "type": "http",
                            "method": "POST",
                            "path": "/seed",
                            "body": "seed=true",
                            "headers": {
                                "Authorization": "Bearer demo"
                            },
                            "expectedStatus": 202
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "seed"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: http POST /seed headers=1 body=9b expect=202"));
    assert!(rendered.contains("Status code: 202"));
    assert!(rendered.contains("seeded!"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_tcp_actions() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert_eq!(request, "PING\n");
        std::io::Write::write_all(&mut stream, b"PONG\n").expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "tcp",
                "endpoint": format!("tcp://{addr}"),
                "recipes": [
                    {
                        "name": "ping",
                        "description": "Ping the raw socket service",
                        "action": {
                            "type": "tcp",
                            "payload": "PING",
                            "appendNewline": true,
                            "expectSubstring": "PONG",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "ping"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(
        rendered.contains("Action: tcp payload=\"PING\" newline expect=\"PONG\" timeout=500ms")
    );
    assert!(rendered.contains("Address:"));
    assert!(rendered.contains("PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_redis_actions() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert_eq!(request, "*1\r\n$4\r\nPING\r\n");
        std::io::Write::write_all(&mut stream, b"+PONG\r\n").expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "redis",
                "endpoint": format!("tcp://{addr}"),
                "recipes": [
                    {
                        "name": "ping",
                        "description": "Ping the redis service",
                        "action": {
                            "type": "redis",
                            "command": ["PING"],
                            "expectSubstring": "PONG",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "ping"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: redis PING expect=\"PONG\" timeout=500ms"));
    assert!(rendered.contains("Type: simple-string"));
    assert!(rendered.contains("Value: PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_parameter_args() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert_eq!(request, "*2\r\n$3\r\nGET\r\n$5\r\nalpha\r\n");
        std::io::Write::write_all(&mut stream, b"$5\r\nvalue\r\n").expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "redis",
                "endpoint": format!("tcp://{addr}"),
                "recipes": [
                    {
                        "name": "get",
                        "description": "Get one cache entry",
                        "parameters": [
                            {
                                "name": "key",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "redis",
                            "command": ["GET", "{{key}}"],
                            "expectSubstring": "value",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "get",
                "args": {
                    "key": "alpha"
                }
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: redis GET alpha"));
    assert!(rendered.contains("Value: value"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_wait_ready_reports_ready_services() {
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
                "readyPattern": "READY"
            }
        }),
        "/tmp",
        &manager,
    );

    let wait_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_wait_ready",
            "arguments": {
                "jobId": "bg-1",
                "timeoutMs": 2_000
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(wait_result["success"], true);
    let rendered = wait_result["contentItems"][0]["text"]
        .as_str()
        .expect("wait text");
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("ready"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_waits_for_ready_pattern_before_http_call() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert!(request.starts_with("GET /health HTTP/1.1\r\n"));
        std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        )
        .expect("write response");
    });

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
                "protocol": "http",
                "endpoint": format!("http://{addr}"),
                "readyPattern": "READY",
                "recipes": [
                    {
                        "name": "health",
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

    let started = std::time::Instant::now();
    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "health",
                "waitForReadyMs": 2_000
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    assert!(started.elapsed() >= std::time::Duration::from_millis(100));
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Readiness: waited"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}

#[test]
fn workspace_stat_path_reports_type_and_size() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("hello.txt"), "alpha").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_stat_path",
            "arguments": {"path": "hello.txt"}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("Path: hello.txt"));
    assert!(text.contains("Type: file"));
    assert!(text.contains("Size: 5 bytes"));
}

#[test]
fn workspace_read_file_returns_line_numbered_content() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("hello.txt"), "alpha\nbeta\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_read_file",
            "arguments": {"path": "hello.txt", "startLine": 2}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("File: hello.txt"));
    assert!(text.contains("   2 | beta"));
}

#[test]
fn workspace_search_text_returns_matching_lines() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("src.txt"), "alpha\nneedle here\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_search_text",
            "arguments": {"query": "needle"}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("Text matches for `needle`:"));
    assert!(text.contains("src.txt:2: needle here"));
}

#[test]
fn workspace_find_files_returns_relative_paths() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(workspace.path().join("src")).expect("mkdir");
    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn demo() {}\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_find_files",
            "arguments": {"query": "lib"}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("File matches for `lib`:"));
    assert!(text.contains("src/lib.rs"));
}

#[test]
fn workspace_read_file_rejects_escape_outside_workspace() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::NamedTempFile::new().expect("tempfile");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_read_file",
            "arguments": {"path": outside.path()}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], false);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("outside the current workspace"));
}
