use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use crate::Cli;
use crate::background_shells::BackgroundShellManager;

use serde_json::Value;

use super::control::LocalApiCommand;
use super::control::new_command_queue;
use super::events::events_since;
use super::events::new_event_log;
use super::events::publish_snapshot_change_events;
use super::server::HttpRequest;
use super::server::route_request;
use super::server::route_request_with_manager;
use super::server::start_local_api;
use super::snapshot::LocalApiBackgroundShellJob;
use super::snapshot::LocalApiCachedAgentThread;
use super::snapshot::LocalApiCapabilityConsumer;
use super::snapshot::LocalApiCapabilityEntry;
use super::snapshot::LocalApiCapabilityProvider;
use super::snapshot::LocalApiDependencyEdge;
use super::snapshot::LocalApiLiveAgentTask;
use super::snapshot::LocalApiOrchestrationStatus;
use super::snapshot::LocalApiSnapshot;
use super::snapshot::LocalApiTranscriptEntry;
use super::snapshot::LocalApiWorkersSnapshot;

fn sample_snapshot() -> Arc<RwLock<LocalApiSnapshot>> {
    Arc::new(RwLock::new(LocalApiSnapshot {
        session_id: "sess_test".to_string(),
        cwd: "/tmp/repo".to_string(),
        thread_id: Some("thread_123".to_string()),
        active_turn_id: Some("turn_456".to_string()),
        objective: Some("continue".to_string()),
        turn_running: true,
        started_turn_count: 3,
        completed_turn_count: 2,
        active_personality: Some("balanced".to_string()),
        orchestration_status: LocalApiOrchestrationStatus {
            main_agent_state: "blocked".to_string(),
            wait_summary: Some("waiting on agent thread_worker".to_string()),
            blocking_dependencies: 2,
            sidecar_dependencies: 1,
            wait_tasks: 1,
            sidecar_agent_tasks: 1,
            exec_prerequisites: 1,
            exec_sidecars: 1,
            exec_services: 1,
            services_ready: 1,
            services_booting: 0,
            services_untracked: 0,
            services_conflicted: 0,
            service_capabilities: 1,
            service_capability_conflicts: 0,
            capability_dependencies_missing: 0,
            capability_dependencies_booting: 1,
            capability_dependencies_ambiguous: 0,
            live_agent_task_count: 1,
            cached_agent_thread_count: 1,
            background_shell_job_count: 2,
            background_terminal_count: 1,
        },
        orchestration_dependencies: vec![LocalApiDependencyEdge {
            from: "main".to_string(),
            to: "agent:thread_worker".to_string(),
            kind: "wait".to_string(),
            blocking: true,
        }],
        workers: LocalApiWorkersSnapshot {
            main_agent_state: "blocked".to_string(),
            wait_summary: Some("waiting on agent thread_worker".to_string()),
            cached_agent_threads: vec![LocalApiCachedAgentThread {
                id: "thread_worker".to_string(),
                status: "running".to_string(),
                preview: "reviewing code".to_string(),
                updated_at: Some(123),
            }],
            live_agent_tasks: vec![LocalApiLiveAgentTask {
                id: "task_1".to_string(),
                tool: "wait".to_string(),
                status: "inProgress".to_string(),
                sender_thread_id: "thread_main".to_string(),
                receiver_thread_ids: vec!["thread_worker".to_string()],
                prompt: Some("reviewing code".to_string()),
                agent_statuses: [("thread_worker".to_string(), "running".to_string())]
                    .into_iter()
                    .collect(),
            }],
            background_shells: vec![
                LocalApiBackgroundShellJob {
                    id: "bg-1".to_string(),
                    pid: 1234,
                    command: "npm run dev".to_string(),
                    cwd: "/tmp/repo".to_string(),
                    intent: "service".to_string(),
                    label: Some("frontend".to_string()),
                    alias: Some("dev.frontend".to_string()),
                    service_capabilities: vec!["@frontend.dev".to_string()],
                    dependency_capabilities: Vec::new(),
                    service_protocol: Some("http".to_string()),
                    service_endpoint: Some("http://127.0.0.1:3000".to_string()),
                    attach_hint: Some("open browser".to_string()),
                    interaction_recipe_names: vec!["health".to_string()],
                    ready_pattern: Some("ready".to_string()),
                    service_readiness: Some("ready".to_string()),
                    origin: Default::default(),
                    status: "running".to_string(),
                    exit_code: None,
                    total_lines: 12,
                    recent_lines: vec!["ready".to_string()],
                },
                LocalApiBackgroundShellJob {
                    id: "bg-2".to_string(),
                    pid: 4321,
                    command: "cargo test".to_string(),
                    cwd: "/tmp/repo".to_string(),
                    intent: "prerequisite".to_string(),
                    label: None,
                    alias: None,
                    service_capabilities: Vec::new(),
                    dependency_capabilities: vec!["@frontend.dev".to_string()],
                    service_protocol: None,
                    service_endpoint: None,
                    attach_hint: None,
                    interaction_recipe_names: Vec::new(),
                    ready_pattern: None,
                    service_readiness: None,
                    origin: Default::default(),
                    status: "running".to_string(),
                    exit_code: None,
                    total_lines: 3,
                    recent_lines: vec!["building".to_string()],
                },
            ],
            background_terminals: vec![Default::default()],
        },
        capabilities: vec![LocalApiCapabilityEntry {
            capability: "@frontend.dev".to_string(),
            issue: "healthy".to_string(),
            providers: vec![LocalApiCapabilityProvider {
                job_id: "bg-1".to_string(),
                alias: Some("dev.frontend".to_string()),
                label: Some("frontend".to_string()),
                readiness: Some("ready".to_string()),
                protocol: Some("http".to_string()),
                endpoint: Some("http://127.0.0.1:3000".to_string()),
            }],
            consumers: vec![LocalApiCapabilityConsumer {
                job_id: "bg-2".to_string(),
                alias: None,
                label: None,
                blocking: true,
                status: "satisfied".to_string(),
            }],
        }],
        transcript: vec![
            LocalApiTranscriptEntry {
                role: "user".to_string(),
                text: "continue".to_string(),
            },
            LocalApiTranscriptEntry {
                role: "assistant".to_string(),
                text: "working on it".to_string(),
            },
        ],
    }))
}

fn get_request(path: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".to_string(),
        path: path.to_string(),
        headers: Default::default(),
        body: Vec::new(),
    }
}

fn post_json_request(path: &str, body: Value) -> HttpRequest {
    HttpRequest {
        method: "POST".to_string(),
        path: path.to_string(),
        headers: Default::default(),
        body: serde_json::to_vec(&body).expect("serialize body"),
    }
}

fn json_body(response_body: &[u8]) -> Value {
    serde_json::from_slice(response_body).expect("response body should be valid json")
}

fn local_api_test_cli() -> Cli {
    crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: true,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    })
}

fn sample_service_manager() -> BackgroundShellManager {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &serde_json::json!({
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "frontend",
                "capabilities": ["frontend.dev"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "open browser",
                "readyPattern": "READY",
                "recipes": [{
                    "name": "health",
                    "description": "Check health",
                    "action": {
                        "type": "stdin",
                        "text": "status",
                        "appendNewline": true
                    }
                }]
            }),
            "/tmp",
        )
        .expect("start service shell");
    manager
        .send_input_for_operator("bg-1", "READY", true)
        .expect("send ready line");
    for _ in 0..40 {
        let rendered = manager
            .poll_job("bg-1", 0, 20)
            .expect("poll service output");
        if rendered.contains("READY") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    manager
}

#[test]
fn healthz_is_public() {
    let response = route_request(
        &get_request("/healthz"),
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 200);
    assert_eq!(json_body(&response.body)["ok"], Value::Bool(true));
}

#[test]
fn session_requires_auth_when_token_is_configured() {
    let response = route_request(
        &get_request("/api/v1/session"),
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 401);
    assert_eq!(json_body(&response.body)["error"]["code"], "unauthorized");
}

#[test]
fn session_snapshot_is_returned_with_valid_token() {
    let mut request = get_request("/api/v1/session");
    request
        .headers
        .insert("authorization".to_string(), "Bearer secret".to_string());
    let response = route_request(
        &request,
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["thread_id"], "thread_123");
    assert_eq!(body["working"], Value::Bool(true));
    assert_eq!(body["orchestration"]["main_agent_state"], "blocked");
}

#[test]
fn session_id_route_reuses_same_snapshot_payload() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["active_turn_id"], "turn_456");
}

#[test]
fn unknown_session_id_returns_not_found() {
    let response = route_request(
        &get_request("/api/v1/session/sess_other"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "session_not_found"
    );
}

#[test]
fn turn_start_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/start",
            serde_json::json!({
                "session_id": "sess_test",
                "input": { "text": "review this diff" }
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(json_body(&response.body)["accepted"], Value::Bool(true));
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::StartTurn {
            session_id: "sess_test".to_string(),
            prompt: "review this diff".to_string(),
        })
    );
}

#[test]
fn turn_start_requires_attached_thread() {
    let snapshot = Arc::new(RwLock::new(LocalApiSnapshot {
        thread_id: None,
        ..sample_snapshot().read().expect("snapshot").clone()
    }));
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/start",
            serde_json::json!({
                "session_id": "sess_test",
                "input": { "text": "review this diff" }
            }),
        ),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "thread_not_attached"
    );
}

#[test]
fn turn_interrupt_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/interrupt",
            serde_json::json!({ "session_id": "sess_test" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::InterruptTurn {
            session_id: "sess_test".to_string(),
        })
    );
}

#[test]
fn orchestration_status_route_returns_structured_counts() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/orchestration/status"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["orchestration"]["main_agent_state"], "blocked");
    assert_eq!(body["orchestration"]["background_shell_job_count"], 2);
}

#[test]
fn orchestration_dependencies_route_returns_edges() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/orchestration/dependencies"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["dependencies"][0]["from"], "main");
    assert_eq!(body["dependencies"][0]["blocking"], Value::Bool(true));
}

#[test]
fn orchestration_workers_route_returns_live_and_cached_workers() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/orchestration/workers"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(
        body["workers"]["cached_agent_threads"][0]["id"],
        "thread_worker"
    );
    assert_eq!(body["workers"]["background_shells"][0]["id"], "bg-1");
}

#[test]
fn shells_services_and_capabilities_routes_return_filtered_views() {
    let snapshot = sample_snapshot();

    let shells = route_request(
        &get_request("/api/v1/session/sess_test/shells"),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(shells.status, 200);
    assert_eq!(
        json_body(&shells.body)["shells"].as_array().map(Vec::len),
        Some(2)
    );

    let services = route_request(
        &get_request("/api/v1/session/sess_test/services"),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(services.status, 200);
    let services_body = json_body(&services.body);
    assert_eq!(services_body["services"].as_array().map(Vec::len), Some(1));
    assert_eq!(services_body["services"][0]["intent"], "service");

    let capabilities = route_request(
        &get_request("/api/v1/session/sess_test/capabilities"),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(capabilities.status, 200);
    let capabilities_body = json_body(&capabilities.body);
    assert_eq!(
        capabilities_body["capabilities"][0]["capability"],
        "@frontend.dev"
    );
    assert_eq!(
        capabilities_body["capabilities"][0]["providers"][0]["job_id"],
        "bg-1"
    );
}

#[test]
fn transcript_route_returns_semantic_conversation_entries() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/transcript"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["transcript"].as_array().map(Vec::len), Some(2));
    assert_eq!(body["transcript"][0]["role"], "user");
    assert_eq!(body["transcript"][1]["role"], "assistant");
}

#[test]
fn shell_start_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/start",
            serde_json::json!({
                "command": "npm run dev",
                "intent": "service",
                "label": "frontend",
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::StartShell {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "command": "npm run dev",
                "intent": "service",
                "label": "frontend",
            }),
        })
    );
}

#[test]
fn shell_poll_route_returns_selected_shell_snapshot() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/dev.frontend/poll",
            serde_json::json!({}),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["shell"]["id"], "bg-1");
    assert_eq!(body["shell"]["alias"], "dev.frontend");
}

#[test]
fn shell_send_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/dev.frontend/send",
            serde_json::json!({
                "text": "status",
                "appendNewline": false,
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::SendShellInput {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "text": "status",
                "appendNewline": false,
            }),
        })
    );
}

#[test]
fn shell_terminate_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/1/terminate",
            serde_json::json!({}),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::TerminateShell {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
            }),
        })
    );
}

#[test]
fn service_update_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/update",
            serde_json::json!({
                "jobId": "bg-1",
                "label": "frontend service",
                "capabilities": ["frontend.dev"]
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "label": "frontend service",
                "capabilities": ["frontend.dev"]
            }),
        })
    );
}

#[test]
fn service_attach_route_returns_attachment_summary() {
    let manager = sample_service_manager();
    let response = route_request_with_manager(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/attach",
            serde_json::json!({}),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        &manager,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    let attachment = body["attachment"]
        .as_str()
        .expect("attachment summary should be a string");
    assert!(attachment.contains("Endpoint: http://127.0.0.1:3000"));
    assert!(attachment.contains("health"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_wait_route_returns_ready_status() {
    let manager = sample_service_manager();
    let response = route_request_with_manager(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/wait",
            serde_json::json!({
                "timeoutMs": 2000
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        &manager,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    let result = body["result"]
        .as_str()
        .expect("wait result should be a string");
    assert!(result.contains("already ready") || result.contains("became ready"));
    assert!(result.contains("Ready pattern: READY"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_run_route_invokes_service_recipe() {
    let manager = sample_service_manager();
    let response = route_request_with_manager(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/run",
            serde_json::json!({
                "recipe": "health"
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        &manager,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    let result = body["result"]
        .as_str()
        .expect("run result should be a string");
    assert!(result.contains("Invoked recipe `health`"));
    assert!(result.contains("Action: stdin \"status\""));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_provide_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/dev.frontend/provide",
            serde_json::json!({
                "capabilities": ["frontend.dev"]
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "capabilities": ["frontend.dev"]
            }),
        })
    );
}

#[test]
fn service_contract_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/contract",
            serde_json::json!({
                "endpoint": "http://127.0.0.1:3001",
                "readyPattern": "listening",
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "endpoint": "http://127.0.0.1:3001",
                "readyPattern": "listening",
            }),
        })
    );
}

#[test]
fn service_relabel_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/@frontend.dev/relabel",
            serde_json::json!({
                "label": "frontend service"
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "label": "frontend service"
            }),
        })
    );
}

#[test]
fn dependency_update_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/dependencies/update",
            serde_json::json!({
                "jobId": "bg-2",
                "dependsOnCapabilities": ["frontend.dev"]
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateDependencies {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-2",
                "dependsOnCapabilities": ["frontend.dev"]
            }),
        })
    );
}

#[test]
fn service_depend_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-2/depend",
            serde_json::json!({
                "dependsOnCapabilities": ["frontend.dev"]
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateDependencies {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-2",
                "dependsOnCapabilities": ["frontend.dev"]
            }),
        })
    );
}

#[test]
fn service_contract_route_requires_contract_fields() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/contract",
            serde_json::json!({}),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "validation_error"
    );
}

#[test]
fn service_contract_route_requires_contract_field() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/contract",
            serde_json::json!({}),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "validation_error"
    );
}

#[test]
fn publish_snapshot_change_events_emits_replayable_semantic_events() {
    let snapshot = sample_snapshot();
    let current = snapshot.read().expect("snapshot").clone();
    let log = new_event_log();
    publish_snapshot_change_events(&log, None, &current);

    let events = events_since(&log, "sess_test", None);
    assert_eq!(events.len(), 6);
    assert_eq!(events[0].event, "session.updated");
    assert_eq!(events[1].event, "turn.updated");
    assert_eq!(events[2].event, "orchestration.updated");
    assert_eq!(events[3].event, "workers.updated");
    assert_eq!(events[4].event, "capabilities.updated");
    assert_eq!(events[5].event, "transcript.updated");
}

#[test]
fn event_stream_route_replays_existing_events() {
    let snapshot = sample_snapshot();
    let current = snapshot.read().expect("snapshot").clone();
    let queue = new_command_queue();
    let log = new_event_log();
    publish_snapshot_change_events(&log, None, &current);

    let handle = start_local_api(
        &local_api_test_cli(),
        snapshot.clone(),
        queue,
        BackgroundShellManager::default(),
        log,
    )
    .expect("start local api")
    .expect("local api enabled");
    let addr = handle.bind_addr().to_string();

    let mut stream = TcpStream::connect(&addr).expect("connect local api");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set read timeout");
    stream
        .write_all(b"GET /api/v1/session/sess_test/events HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("write request");

    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = stream.read(&mut buffer).expect("read response");
        if read == 0 {
            break;
        }
        response.extend_from_slice(&buffer[..read]);
        let response_text = String::from_utf8_lossy(&response);
        if response_text.contains("event: session.updated")
            && response_text.contains("event: turn.updated")
        {
            break;
        }
    }
    let response_text = String::from_utf8_lossy(&response);
    assert!(response_text.contains("HTTP/1.1 200 OK"));
    assert!(response_text.contains("Content-Type: text/event-stream"));
    assert!(response_text.contains("event: session.updated"));
    assert!(response_text.contains("event: turn.updated"));

    drop(stream);
    handle.shutdown().expect("shutdown local api");
}
