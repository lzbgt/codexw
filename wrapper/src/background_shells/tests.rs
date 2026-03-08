use super::BackgroundShellIntent;
use super::BackgroundShellManager;
use super::BackgroundShellOrigin;
use super::BackgroundShellServiceReadiness;
use serde_json::json;
use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[cfg(unix)]
fn interactive_echo_command() -> &'static str {
    "cat"
}

#[cfg(windows)]
fn interactive_echo_command() -> &'static str {
    "more"
}

#[cfg(unix)]
fn service_ready_command() -> &'static str {
    "printf 'booting\\nREADY\\n'; sleep 0.4"
}

#[cfg(windows)]
fn service_ready_command() -> &'static str {
    "echo booting && echo READY && ping -n 2 127.0.0.1 >NUL"
}

#[cfg(unix)]
fn delayed_service_ready_command() -> &'static str {
    "sleep 0.15; printf 'READY\\n'; sleep 0.4"
}

#[cfg(windows)]
fn delayed_service_ready_command() -> &'static str {
    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
}

fn spawn_test_http_server(
    expected_method: &'static str,
    expected_path: &'static str,
    response_body: &'static str,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        let first_line = request.lines().next().expect("request line");
        assert_eq!(
            first_line,
            format!("{expected_method} {expected_path} HTTP/1.1")
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });
    format!("http://{addr}")
}

fn spawn_test_http_server_with_assertions(
    assert_request: impl FnOnce(&str) + Send + 'static,
    response: &'static str,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]).into_owned();
        assert_request(&request);
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });
    format!("http://{addr}")
}

#[test]
fn background_shell_job_can_start_and_poll_output() {
    let manager = BackgroundShellManager::default();
    let started = manager
        .start_from_tool(&json!({"command": "printf 'alpha\\nbeta\\n'"}), "/tmp")
        .expect("start background shell");
    assert!(started.contains("Started background shell job bg-1"));

    let mut rendered = String::new();
    for _ in 0..20 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll background shell");
        if rendered.contains("alpha") && rendered.contains("beta") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("Job: bg-1"));
    assert!(rendered.contains("alpha"));
    assert!(rendered.contains("beta"));
}

#[test]
fn background_shell_job_accepts_stdin_and_emits_output() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": interactive_echo_command()}), "/tmp")
        .expect("start interactive background shell");

    manager
        .send_input_for_operator("bg-1", "hello from stdin", true)
        .expect("send stdin");

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll background shell");
        if rendered.contains("hello from stdin") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("hello from stdin"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_list_reports_running_jobs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start background shell");
    let rendered = manager.list_from_tool();
    assert!(rendered.contains("Background shell jobs:"));
    assert!(rendered.contains("bg-1"));
    assert!(rendered.contains("running"));
    assert!(rendered.contains("intent=observation"));
    let _ = manager.terminate_all_running();
}

#[test]
fn running_service_capabilities_can_be_reassigned_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service");

    let updated = manager
        .set_running_service_capabilities("bg-1", &["frontend.dev".to_string()])
        .expect("update capabilities");
    assert_eq!(updated, vec!["frontend.dev".to_string()]);

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(!rendered.contains("@api.http"));
    assert!(rendered.contains("@frontend.dev"));
    let _ = manager.terminate_all_running();
}

#[test]
fn running_service_label_can_be_updated_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service");

    let updated = manager
        .set_running_service_label("bg-1", Some("frontend dev".to_string()))
        .expect("update label");
    assert_eq!(updated, Some("frontend dev".to_string()));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated service");
    assert!(rendered.contains("Label: frontend dev"));

    let cleared = manager
        .set_running_service_label("bg-1", None)
        .expect("clear label");
    assert_eq!(cleared, None);

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll cleared label");
    assert!(!rendered.contains("Label: frontend dev"));
    let _ = manager.terminate_all_running();
}

#[test]
fn running_dependency_capabilities_can_be_updated_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start blocker");

    let updated = manager
        .set_running_dependency_capabilities("bg-1", &["db.redis".to_string()])
        .expect("update dependency capabilities");
    assert_eq!(updated, vec!["db.redis".to_string()]);

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated blocker");
    assert!(rendered.contains("Depends on capabilities: @db.redis"));
    assert!(!rendered.contains("Depends on capabilities: @api.http"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_shell_views_can_filter_ready_booting_untracked_and_conflicting_jobs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": delayed_service_ready_command(),
                "intent": "service",
                "label": "booting svc",
                "capabilities": ["svc.booting"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");
    manager
        .start_from_tool(
            &json!({
                "command": service_ready_command(),
                "intent": "service",
                "label": "ready svc",
                "capabilities": ["svc.ready"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready service");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "untracked svc",
                "capabilities": ["svc.untracked"]
            }),
            "/tmp",
        )
        .expect("start untracked service");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "conflict a",
                "capabilities": ["svc.conflict"]
            }),
            "/tmp",
        )
        .expect("start first conflicting service");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "conflict b",
                "capabilities": ["svc.conflict"]
            }),
            "/tmp",
        )
        .expect("start second conflicting service");

    manager
        .wait_ready_for_operator("bg-2", 2_000)
        .expect("wait for ready service");

    let ready = manager
        .render_service_shells_for_ps_filtered(
            Some(super::BackgroundShellServiceIssueClass::Ready),
            None,
        )
        .expect("ready service render")
        .join("\n");
    assert!(ready.contains("ready svc"));
    assert!(!ready.contains("booting svc"));

    let booting = manager
        .render_service_shells_for_ps_filtered(
            Some(super::BackgroundShellServiceIssueClass::Booting),
            None,
        )
        .expect("booting service render")
        .join("\n");
    assert!(booting.contains("booting svc"));
    assert!(!booting.contains("ready svc"));

    let untracked = manager
        .render_service_shells_for_ps_filtered(
            Some(super::BackgroundShellServiceIssueClass::Untracked),
            None,
        )
        .expect("untracked service render")
        .join("\n");
    assert!(untracked.contains("untracked svc"));
    assert!(!untracked.contains("booting svc"));

    let conflicts = manager
        .render_service_shells_for_ps_filtered(
            Some(super::BackgroundShellServiceIssueClass::Conflicts),
            None,
        )
        .expect("conflicting service render")
        .join("\n");
    assert!(conflicts.contains("conflict a"));
    assert!(conflicts.contains("conflict b"));
    assert!(conflicts.contains("Capability conflicts:"));
    assert!(conflicts.contains("@svc.conflict ->"));
    assert!(!conflicts.contains("ready svc"));
    let _ = manager.terminate_all_running();
}

#[test]
fn terminate_running_services_by_capability_terminates_all_matching_providers() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "api a",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first api provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "api b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second api provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "db",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start unrelated provider");

    let terminated = manager
        .terminate_running_services_by_capability("@api.http")
        .expect("terminate api providers");
    assert_eq!(terminated, 2);

    let rendered = manager
        .render_service_shells_for_ps_filtered(None, None)
        .expect("render remaining services")
        .join("\n");
    assert!(!rendered.contains("api a"));
    assert!(!rendered.contains("api b"));
    assert!(rendered.contains("db"));

    let _ = manager.terminate_all_running();
}

#[test]
fn terminate_running_blockers_by_capability_terminates_only_matching_prereqs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api blocker");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "prerequisite",
                "label": "db blocker",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    let terminated = manager
        .terminate_running_blockers_by_capability("@api.http")
        .expect("terminate api blockers");
    assert_eq!(terminated, 1);

    let rendered = manager
        .capability_dependency_summaries()
        .into_iter()
        .map(|summary| format!("{} -> {}", summary.job_id, summary.capability))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!rendered.contains("api.http"));
    assert!(rendered.contains("db.redis"));

    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_origin_intent_and_label_are_preserved_in_snapshots_and_poll() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool_with_context(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "webpack dev server",
                "capabilities": ["web.dev", "frontend"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "Open the dev server in a browser",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
                        "example": "curl http://127.0.0.1:3000/health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
            BackgroundShellOrigin {
                source_thread_id: Some("thread-agent-1".to_string()),
                source_call_id: Some("call-77".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        )
        .expect("start background shell");

    let snapshots = manager.snapshots();
    assert_eq!(
        snapshots[0].origin.source_thread_id.as_deref(),
        Some("thread-agent-1")
    );
    assert_eq!(snapshots[0].intent, BackgroundShellIntent::Service);
    assert_eq!(snapshots[0].label.as_deref(), Some("webpack dev server"));
    assert_eq!(
        snapshots[0].service_capabilities,
        vec!["frontend".to_string(), "web.dev".to_string()]
    );
    assert_eq!(snapshots[0].service_protocol.as_deref(), Some("http"));
    assert_eq!(
        snapshots[0].service_endpoint.as_deref(),
        Some("http://127.0.0.1:3000")
    );
    assert_eq!(
        snapshots[0].attach_hint.as_deref(),
        Some("Open the dev server in a browser")
    );
    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll background shell");
    assert!(rendered.contains("Intent: service"));
    assert!(rendered.contains("Label: webpack dev server"));
    assert!(rendered.contains("Capabilities: frontend, web.dev"));
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:3000"));
    assert!(rendered.contains("Attach hint: Open the dev server in a browser"));
    assert!(rendered.contains("Recipes:"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    assert!(rendered.contains("Source thread: thread-agent-1"));
    assert!(rendered.contains("Source call: call-77"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_capability_reference_resolves_unique_service_job() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api"]
            }),
            "/tmp",
        )
        .expect("start service shell");
    assert_eq!(
        manager
            .resolve_job_reference("@api")
            .expect("resolve capability"),
        "bg-1"
    );
    let resolved = manager
        .resolve_job_reference("@api")
        .expect("resolve capability");
    let attachment = manager
        .attach_for_operator(&resolved)
        .expect("attach by capability");
    assert!(attachment.contains("Capabilities: api"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_capability_reference_errors_when_ambiguous() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api"]
            }),
            "/tmp",
        )
        .expect("start first service shell");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api"]
            }),
            "/tmp",
        )
        .expect("start second service shell");
    let err = manager
        .resolve_job_reference("@api")
        .expect_err("capability should be ambiguous");
    assert!(err.contains("ambiguous"));
    assert!(err.contains("/ps capabilities"));
    assert!(err.contains("bg-1"));
    assert!(err.contains("bg-2"));
    let listed = manager.list_from_tool();
    assert!(listed.contains("Capability index:"));
    assert!(listed.contains("Capability conflicts:"));
    assert!(listed.contains("@api -> bg-1, bg-2"));
    let services = manager
        .render_for_ps_filtered(Some(BackgroundShellIntent::Service))
        .expect("service rendering");
    let rendered = services.join("\n");
    assert!(rendered.contains("Capability conflicts:"));
    assert!(rendered.contains("@api -> bg-1, bg-2"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_capability_reference_ignores_completed_service_jobs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "printf 'done\\n'",
                "intent": "service",
                "capabilities": ["api"]
            }),
            "/tmp",
        )
        .expect("start short-lived service");
    let err = (0..20)
        .find_map(|_| match manager.resolve_job_reference("@api") {
            Ok(_) => {
                thread::sleep(Duration::from_millis(25));
                None
            }
            Err(err) => Some(err),
        })
        .expect("completed service should not satisfy capability resolution");
    assert!(err.contains("unknown running background shell capability"));
}

#[test]
fn service_capability_index_lists_running_service_roles() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http", "frontend.dev"]
            }),
            "/tmp",
        )
        .expect("start first service");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http", "worker.queue"]
            }),
            "/tmp",
        )
        .expect("start second service");

    let index = manager.service_capability_index();
    assert_eq!(
        index,
        vec![
            (
                "api.http".to_string(),
                vec!["bg-1".to_string(), "bg-2".to_string()]
            ),
            ("frontend.dev".to_string(), vec!["bg-1".to_string()]),
            ("worker.queue".to_string(), vec!["bg-2".to_string()]),
        ]
    );

    let rendered = manager
        .render_service_capabilities_for_ps()
        .expect("render capability index")
        .join("\n");
    assert!(rendered.contains("Service capability index:"));
    assert!(rendered.contains("@api.http -> bg-1, bg-2 [conflict]"));
    assert!(rendered.contains("@frontend.dev -> bg-1"));
    assert!(rendered.contains("@worker.queue -> bg-2"));
    let _ = manager.terminate_all_running();
}

#[test]
fn capability_index_can_render_consumers_of_reusable_services() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "integration test",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    let rendered = manager
        .render_service_capabilities_for_ps()
        .expect("render capability index")
        .join("\n");
    assert!(rendered.contains("@api.http -> bg-1"));
    assert!(rendered.contains("used by bg-2 (integration test) [satisfied]"));
    let polled = manager
        .poll_job("bg-2", 0, 20)
        .expect("poll dependent shell");
    assert!(polled.contains("Depends on capabilities: @api.http"));
    let _ = manager.terminate_all_running();
}

#[test]
fn capability_index_can_render_missing_providers_for_declared_dependencies() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    let rendered = manager
        .render_service_capabilities_for_ps()
        .expect("render capability index")
        .join("\n");
    assert!(rendered.contains("@api.http -> <missing provider>"));
    assert!(rendered.contains("used by bg-1 [missing]"));
    let _ = manager.terminate_all_running();
}

#[test]
fn single_capability_view_renders_providers_and_consumers() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start provider");
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "integration test",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start dependent");

    let rendered = manager
        .render_single_service_capability_for_ps("@api.http")
        .expect("render capability detail")
        .join("\n");
    assert!(rendered.contains("Service capability: @api.http"));
    assert!(rendered.contains("Providers:"));
    assert!(rendered.contains("bg-1  [untracked]"));
    assert!(rendered.contains("bg-2 (integration test)  [satisfied]  blocking=yes"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_shell_ready_pattern_transitions_from_booting_to_ready() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": service_ready_command(),
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready-pattern service shell");

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll service shell");
        if rendered.contains("Service state: ready") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("Service state: ready"));
    assert_eq!(
        manager.running_service_count_by_readiness(BackgroundShellServiceReadiness::Ready),
        1
    );
    let _ = manager.terminate_all_running();
}

#[test]
fn wait_ready_for_operator_reports_service_readiness() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": delayed_service_ready_command(),
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready-pattern service shell");

    let rendered = manager
        .wait_ready_for_operator("bg-1", 2_000)
        .expect("wait for service readiness");
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("ready"));
    let _ = manager.terminate_all_running();
}

#[test]
fn wait_ready_for_operator_rejects_untracked_services() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.2",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start untracked service shell");

    let err = manager
        .wait_ready_for_operator("bg-1", 500)
        .expect_err("untracked service should reject ready wait");
    assert!(err.contains("does not declare a `readyPattern`"));
    let _ = manager.terminate_all_running();
}

#[test]
fn ready_pattern_requires_service_intent() {
    let manager = BackgroundShellManager::default();
    let err = manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.1",
                "intent": "observation",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect_err("readyPattern should require service intent");
    assert!(err.contains("readyPattern"));
    assert_eq!(manager.job_count(), 0);
}

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
fn service_recipe_can_invoke_stdin_action() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": interactive_echo_command(),
                "intent": "service",
                "recipes": [
                    {
                        "name": "status",
                        "description": "Ask the service for status",
                        "action": {
                            "type": "stdin",
                            "text": "status"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "status")
        .expect("invoke stdin recipe");
    assert!(rendered.contains("Action: stdin \"status\""));
    assert!(rendered.contains("Sent"));

    let mut polled = String::new();
    for _ in 0..40 {
        polled = manager
            .poll_job("bg-1", 0, 200)
            .expect("poll shell directly");
        if polled.contains("status") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(polled.contains("status"));
    let _ = manager.terminate_all_running();
}

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

#[test]
fn service_recipe_can_invoke_tcp_action() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]).into_owned();
        assert_eq!(request, "PING\n");
        stream.write_all(b"PONG\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
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
            }),
            "/tmp",
        )
        .expect("start tcp service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect("invoke tcp recipe");
    assert!(
        rendered.contains("Action: tcp payload=\"PING\" newline expect=\"PONG\" timeout=500ms")
    );
    assert!(rendered.contains("Address:"));
    assert!(rendered.contains("Payload:"));
    assert!(rendered.contains("PING"));
    assert!(rendered.contains("PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_tcp_expectation_is_enforced() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).expect("read request");
        stream.write_all(b"ERR\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "tcp",
                "endpoint": format!("{addr}"),
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
            }),
            "/tmp",
        )
        .expect("start tcp service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect_err("expectation mismatch should fail");
    assert!(err.contains("expected substring `PONG`"));
    assert!(err.contains("ERR"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_can_invoke_redis_action() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]).into_owned();
        assert_eq!(request, "*1\r\n$4\r\nPING\r\n");
        stream.write_all(b"+PONG\r\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
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
            }),
            "/tmp",
        )
        .expect("start redis service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect("invoke redis recipe");
    assert!(rendered.contains("Action: redis PING expect=\"PONG\" timeout=500ms"));
    assert!(rendered.contains("Command: PING"));
    assert!(rendered.contains("Type: simple-string"));
    assert!(rendered.contains("Value: PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_redis_expectation_is_enforced() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).expect("read request");
        stream.write_all(b"+NOPE\r\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "redis",
                "endpoint": format!("{addr}"),
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
            }),
            "/tmp",
        )
        .expect("start redis service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect_err("expectation mismatch should fail");
    assert!(err.contains("expected substring `PONG`"));
    assert!(err.contains("Value: NOPE"));
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
                        "description": "Read the service docs first"
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "docs")
        .expect_err("informational recipe should not be invokable");
    assert!(err.contains("descriptive only"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_attachment_fields_require_service_intent() {
    let manager = BackgroundShellManager::default();
    let err = manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.1",
                "intent": "observation",
                "protocol": "http"
            }),
            "/tmp",
        )
        .expect_err("service attachment fields should require service intent");
    assert!(err.contains("service contract fields"));
}

#[test]
fn background_shell_manager_counts_running_jobs_by_intent() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite background shell");
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service background shell");
    manager
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start observation background shell");

    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Prerequisite),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Service),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Observation),
        1
    );
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_manager_can_terminate_only_selected_intent() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite background shell");
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service background shell");
    manager
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start observation background shell");

    assert_eq!(
        manager.terminate_running_by_intent(BackgroundShellIntent::Service),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Service),
        0
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Prerequisite),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Observation),
        1
    );
    let _ = manager.terminate_all_running();
}

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
