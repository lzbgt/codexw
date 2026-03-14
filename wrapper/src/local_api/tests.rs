mod events;
mod orchestration;
mod service_routes;
mod session;
mod shell_routes;
mod turn;

use std::collections::BTreeMap;
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
pub(super) use super::server::route_request;
pub(super) use super::server::route_request_with_manager;
pub(super) use super::server::start_local_api;
use super::snapshot::LocalApiAsyncToolBackpressure;
use super::snapshot::LocalApiAsyncToolSupervision;
use super::snapshot::LocalApiAsyncToolWorker;
use super::snapshot::LocalApiBackgroundShellJob;
use super::snapshot::LocalApiBackgroundShellOrigin;
use super::snapshot::LocalApiBackgroundTerminal;
use super::snapshot::LocalApiCachedAgentThread;
use super::snapshot::LocalApiCapabilityConsumer;
use super::snapshot::LocalApiCapabilityEntry;
use super::snapshot::LocalApiCapabilityProvider;
use super::snapshot::LocalApiDependencyEdge;
use super::snapshot::LocalApiLiveAgentTask;
use super::snapshot::LocalApiObservedBackgroundShellJob;
use super::snapshot::LocalApiOrchestrationStatus;
use super::snapshot::LocalApiRecoveryOption;
use super::snapshot::LocalApiRecoveryPolicy;
use super::snapshot::LocalApiSnapshot;
use super::snapshot::LocalApiSupervisionNotice;
use super::snapshot::LocalApiTranscriptEntry;
use super::snapshot::LocalApiWorkersSnapshot;

pub(super) fn sample_snapshot() -> Arc<RwLock<LocalApiSnapshot>> {
    Arc::new(RwLock::new(LocalApiSnapshot {
        session_id: "sess_test".to_string(),
        cwd: "/tmp/repo".to_string(),
        attachment_client_id: Some("client_web".to_string()),
        attachment_lease_seconds: Some(300),
        attachment_lease_expires_at_ms: Some(4_102_444_800_000),
        thread_id: Some("thread_123".to_string()),
        active_turn_id: Some("turn_456".to_string()),
        objective: Some("continue".to_string()),
        turn_running: true,
        started_turn_count: 3,
        completed_turn_count: 2,
        active_personality: Some("balanced".to_string()),
        async_tool_supervision: Some(LocalApiAsyncToolSupervision {
            classification: "tool_slow".to_string(),
            recommended_action: "observe_or_interrupt".to_string(),
            recovery_policy: LocalApiRecoveryPolicy {
                kind: "warn_only".to_string(),
                automation_ready: false,
            },
            recovery_options: vec![
                LocalApiRecoveryOption {
                    kind: "observe_status".to_string(),
                    label: "Observe current session status".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("GET".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test".to_string()),
                },
                LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
                },
            ],
            request_id: "7".to_string(),
            thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
            owner: "wrapper_background_shell".to_string(),
            source_call_id: Some("call_1".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-1".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: "wrapper_background_shell_streaming_output".to_string(),
            output_state: "recent_output_observed".to_string(),
            observed_background_shell_job: Some(LocalApiObservedBackgroundShellJob {
                job_id: "bg-1".to_string(),
                status: "running".to_string(),
                command: "npm run dev".to_string(),
                total_lines: 1,
                last_output_age_seconds: Some(2),
                recent_lines: vec!["READY".to_string()],
            }),
            next_check_in_seconds: 9,
            elapsed_seconds: 21,
            active_request_count: 1,
        }),
        async_tool_backpressure: Some(LocalApiAsyncToolBackpressure {
            abandoned_request_count: 1,
            saturation_threshold: crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
            saturated: false,
            recovery_options: vec![
                LocalApiRecoveryOption {
                    kind: "observe_status".to_string(),
                    label: "Observe current session status".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("GET".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test".to_string()),
                },
                LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
                },
                LocalApiRecoveryOption {
                    kind: "exit_and_resume".to_string(),
                    label: "Exit and resume the thread in a newer client".to_string(),
                    automation_ready: false,
                    cli_command: Some("codexw --cwd /tmp/repo resume thread_123".to_string()),
                    local_api_method: None,
                    local_api_path: None,
                },
            ],
            oldest_request_id: "8".to_string(),
            oldest_thread_name: "codexw-bgtool-background_shell_start-8".to_string(),
            oldest_tool: "background_shell_start".to_string(),
            oldest_summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            oldest_source_call_id: Some("call_2".to_string()),
            oldest_target_background_shell_reference: Some("dev.api".to_string()),
            oldest_target_background_shell_job_id: Some("bg-1".to_string()),
            oldest_observation_state: "wrapper_background_shell_streaming_output".to_string(),
            oldest_output_state: "recent_output_observed".to_string(),
            oldest_observed_background_shell_job: Some(LocalApiObservedBackgroundShellJob {
                job_id: "bg-1".to_string(),
                status: "running".to_string(),
                command: "npm run dev".to_string(),
                total_lines: 1,
                last_output_age_seconds: Some(2),
                recent_lines: vec!["READY".to_string()],
            }),
            oldest_elapsed_before_timeout_seconds: 21,
            oldest_hard_timeout_seconds: 15,
            oldest_elapsed_seconds: 6,
        }),
        async_tool_workers: vec![
            LocalApiAsyncToolWorker {
                request_id: "7".to_string(),
                lifecycle_state: "running".to_string(),
                thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
                owner: "wrapper_background_shell".to_string(),
                source_call_id: Some("call_1".to_string()),
                target_background_shell_reference: Some("dev.api".to_string()),
                target_background_shell_job_id: Some("bg-1".to_string()),
                tool: "background_shell_start".to_string(),
                summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
                observation_state: Some("wrapper_background_shell_streaming_output".to_string()),
                output_state: Some("recent_output_observed".to_string()),
                observed_background_shell_job: Some(LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "running".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 1,
                    last_output_age_seconds: Some(2),
                    recent_lines: vec!["READY".to_string()],
                }),
                next_check_in_seconds: Some(9),
                runtime_elapsed_seconds: 21,
                state_elapsed_seconds: 21,
                hard_timeout_seconds: 15,
                supervision_classification: Some("tool_slow".to_string()),
            },
            LocalApiAsyncToolWorker {
                request_id: "8".to_string(),
                lifecycle_state: "abandoned_after_timeout".to_string(),
                thread_name: "codexw-bgtool-background_shell_start-8".to_string(),
                owner: "wrapper_background_shell".to_string(),
                source_call_id: Some("call_2".to_string()),
                target_background_shell_reference: Some("dev.api".to_string()),
                target_background_shell_job_id: Some("bg-1".to_string()),
                tool: "background_shell_start".to_string(),
                summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
                observation_state: Some("wrapper_background_shell_streaming_output".to_string()),
                output_state: Some("recent_output_observed".to_string()),
                observed_background_shell_job: Some(LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "running".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 1,
                    last_output_age_seconds: Some(2),
                    recent_lines: vec!["READY".to_string()],
                }),
                next_check_in_seconds: None,
                runtime_elapsed_seconds: 21,
                state_elapsed_seconds: 6,
                hard_timeout_seconds: 15,
                supervision_classification: None,
            },
        ],
        supervision_notice: Some(LocalApiSupervisionNotice {
            classification: "tool_slow".to_string(),
            recommended_action: "observe_or_interrupt".to_string(),
            recovery_policy: LocalApiRecoveryPolicy {
                kind: "warn_only".to_string(),
                automation_ready: false,
            },
            recovery_options: vec![
                LocalApiRecoveryOption {
                    kind: "observe_status".to_string(),
                    label: "Observe current session status".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("GET".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test".to_string()),
                },
                LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
                },
            ],
            request_id: "7".to_string(),
            thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
            owner: "wrapper_background_shell".to_string(),
            source_call_id: Some("call_1".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-1".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: "wrapper_background_shell_streaming_output".to_string(),
            output_state: "recent_output_observed".to_string(),
            observed_background_shell_job: Some(LocalApiObservedBackgroundShellJob {
                job_id: "bg-1".to_string(),
                status: "running".to_string(),
                command: "npm run dev".to_string(),
                total_lines: 1,
                last_output_age_seconds: Some(2),
                recent_lines: vec!["READY".to_string()],
            }),
        }),
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
                id: "item_worker".to_string(),
                tool: "spawn_agent".to_string(),
                status: "running".to_string(),
                sender_thread_id: "thread_main".to_string(),
                receiver_thread_ids: vec!["thread_worker".to_string()],
                prompt: Some("Reviewing code".to_string()),
                agent_statuses: BTreeMap::from([(
                    "thread_worker".to_string(),
                    "running".to_string(),
                )]),
            }],
            background_shells: vec![
                LocalApiBackgroundShellJob {
                    id: "bg-1".to_string(),
                    pid: 1234,
                    command: "npm run dev".to_string(),
                    cwd: "/tmp/repo".to_string(),
                    alias: Some("dev.frontend".to_string()),
                    label: Some("frontend".to_string()),
                    intent: "service".to_string(),
                    service_capabilities: vec!["@frontend.dev".to_string()],
                    dependency_capabilities: Vec::new(),
                    service_protocol: Some("http".to_string()),
                    service_endpoint: Some("http://127.0.0.1:3000".to_string()),
                    attach_hint: Some("open browser".to_string()),
                    interaction_recipe_names: vec!["health".to_string()],
                    ready_pattern: Some("READY".to_string()),
                    service_readiness: Some("ready".to_string()),
                    origin: LocalApiBackgroundShellOrigin {
                        source_thread_id: Some("thread_main".to_string()),
                        source_call_id: Some("call_1".to_string()),
                        source_tool: Some("background_shell_start".to_string()),
                    },
                    status: "running".to_string(),
                    exit_code: None,
                    total_lines: 1,
                    last_output_age_seconds: Some(2),
                    recent_lines: vec!["READY".to_string()],
                },
                LocalApiBackgroundShellJob {
                    id: "bg-2".to_string(),
                    pid: 2345,
                    command: "npm test".to_string(),
                    cwd: "/tmp/repo".to_string(),
                    alias: None,
                    label: Some("tests".to_string()),
                    intent: "observation".to_string(),
                    service_capabilities: Vec::new(),
                    dependency_capabilities: vec!["@frontend.dev".to_string()],
                    service_protocol: None,
                    service_endpoint: None,
                    attach_hint: None,
                    interaction_recipe_names: Vec::new(),
                    ready_pattern: None,
                    service_readiness: None,
                    origin: LocalApiBackgroundShellOrigin {
                        source_thread_id: Some("thread_main".to_string()),
                        source_call_id: Some("call_2".to_string()),
                        source_tool: Some("background_shell_start".to_string()),
                    },
                    status: "running".to_string(),
                    exit_code: None,
                    total_lines: 1,
                    last_output_age_seconds: Some(10),
                    recent_lines: vec!["running".to_string()],
                },
            ],
            background_terminals: vec![LocalApiBackgroundTerminal {
                item_id: "item-term-1".to_string(),
                process_id: "term-1".to_string(),
                command_display: "npm run dev".to_string(),
                waiting: false,
                recent_inputs: Vec::new(),
                recent_output: vec!["ready".to_string()],
            }],
        },
        capabilities: vec![LocalApiCapabilityEntry {
            capability: "@frontend.dev".to_string(),
            issue: "healthy".to_string(),
            providers: vec![LocalApiCapabilityProvider {
                job_id: "bg-1".to_string(),
                alias: Some("dev.frontend".to_string()),
                label: Some("frontend".to_string()),
                readiness: Some("ready".to_string()),
                endpoint: Some("http://127.0.0.1:3000".to_string()),
                protocol: Some("http".to_string()),
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

pub(super) fn get_request(path: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".to_string(),
        path: path.to_string(),
        headers: Default::default(),
        body: Vec::new(),
    }
}

pub(super) fn post_json_request(path: &str, body: Value) -> HttpRequest {
    HttpRequest {
        method: "POST".to_string(),
        path: path.to_string(),
        headers: Default::default(),
        body: serde_json::to_vec(&body).expect("serialize body"),
    }
}

pub(super) fn json_body(response_body: &[u8]) -> Value {
    serde_json::from_slice(response_body).expect("response body should be valid json")
}

pub(super) fn local_api_test_cli() -> Cli {
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

pub(super) fn sample_service_manager() -> BackgroundShellManager {
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
