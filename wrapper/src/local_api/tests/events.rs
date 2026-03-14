use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;
use crate::background_shells::BackgroundShellManager;

use super::events_since;
use super::json_body;
use super::new_command_queue;
use super::new_event_log;
use super::post_json_request;
use super::publish_snapshot_change_events;
use super::route_request;
use super::sample_snapshot;
use super::start_local_api;
use crate::local_api::publish_client_event;

#[test]
fn publish_snapshot_change_events_emits_replayable_semantic_events() {
    let snapshot = sample_snapshot();
    let current = snapshot.read().expect("snapshot").clone();
    let log = new_event_log();
    publish_snapshot_change_events(&log, None, &current);

    let events = events_since(&log, "sess_test", None);
    assert_eq!(events.len(), 7);
    assert_eq!(events[0].event, "session.updated");
    assert_eq!(
        events[0].data["session"]["attachment"]["id"],
        "attach:sess_test"
    );
    assert_eq!(events[0].data["attachment"]["scope"], "process");
    assert_eq!(events[0].data["attachment"]["client_id"], "client_web");
    assert_eq!(events[0].data["attachment"]["lease_seconds"], 300);
    assert_eq!(events[1].event, "turn.updated");
    assert_eq!(
        events[0].data["session"]["async_tool_supervision"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["classification"],
        "tool_slow"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recovery_policy"]["kind"],
        "warn_only"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recovery_options"][0]["kind"],
        "observe_status"
    );
    assert_eq!(events[2].event, "status.updated");
    assert_eq!(
        events[2].data["async_tool_supervision"]["classification"],
        "tool_slow"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["next_check_in_seconds"],
        9
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["abandoned_request_count"],
        1
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_hard_timeout_seconds"],
        15
    );
    assert_eq!(events[2].data["async_tool_workers"][0]["request_id"], "7");
    assert_eq!(
        events[2].data["async_tool_workers"][0]["lifecycle_state"],
        "running"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["next_check_in_seconds"],
        9
    );
    assert_eq!(
        events[2].data["async_tool_workers"][1]["lifecycle_state"],
        "abandoned_after_timeout"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["recovery_policy"]["automation_ready"],
        false
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["recovery_options"][1]["kind"],
        "interrupt_turn"
    );
    assert_eq!(events[3].event, "orchestration.updated");
    assert_eq!(events[4].event, "workers.updated");
    assert_eq!(events[5].event, "capabilities.updated");
    assert_eq!(events[6].event, "transcript.updated");
}

#[test]
fn event_stream_route_replays_existing_events() {
    let snapshot = sample_snapshot();
    let current = snapshot.read().expect("snapshot").clone();
    let queue = new_command_queue();
    let log = new_event_log();
    publish_snapshot_change_events(&log, None, &current);

    let handle = start_local_api(
        &super::local_api_test_cli(),
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
            && response_text.contains("event: status.updated")
        {
            break;
        }
    }
    let response_text = String::from_utf8_lossy(&response);
    assert!(response_text.contains("HTTP/1.1 200 OK"));
    assert!(response_text.contains("Content-Type: text/event-stream"));
    assert!(response_text.contains(&format!(
        "{}: {}",
        HEADER_LOCAL_API_VERSION, CODEXW_LOCAL_API_VERSION
    )));
    assert!(response_text.contains("event: session.updated"));
    assert!(response_text.contains("event: turn.updated"));
    assert!(response_text.contains("event: status.updated"));
    assert!(response_text.contains("\"recommended_action\":\"observe_or_interrupt\""));
    assert!(response_text.contains("\"async_tool_backpressure\""));
    assert!(response_text.contains("\"async_tool_workers\""));
    assert!(response_text.contains("\"supervision_notice\""));

    drop(stream);
    handle.shutdown().expect("shutdown local api");
}

#[test]
fn publish_snapshot_change_events_emits_status_update_when_supervision_changes() {
    let snapshot = sample_snapshot();
    let previous = snapshot.read().expect("snapshot").clone();
    let mut current = previous.clone();
    current.async_tool_supervision =
        Some(crate::local_api::snapshot::LocalApiAsyncToolSupervision {
            classification: "tool_wedged".to_string(),
            recommended_action: "interrupt_or_exit_resume".to_string(),
            recovery_policy: crate::local_api::snapshot::LocalApiRecoveryPolicy {
                kind: "operator_interrupt_or_exit_resume".to_string(),
                automation_ready: false,
            },
            recovery_options: vec![
                crate::local_api::snapshot::LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
                },
                crate::local_api::snapshot::LocalApiRecoveryOption {
                    kind: "exit_and_resume".to_string(),
                    label: "Exit and resume the thread in a newer client".to_string(),
                    automation_ready: false,
                    cli_command: Some("codexw --cwd /tmp/repo resume thread_123".to_string()),
                    local_api_method: None,
                    local_api_path: None,
                },
            ],
            owner: "wrapper_background_shell".to_string(),
            source_call_id: Some("call_1".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: "wrapper_background_shell_terminal_without_tool_response"
                .to_string(),
            observed_background_shell_job: Some(
                crate::local_api::snapshot::LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "failed".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 3,
                    recent_lines: vec!["boom".to_string()],
                },
            ),
            next_check_in_seconds: 30,
            elapsed_seconds: 75,
            active_request_count: 1,
        });
    current.async_tool_backpressure =
        Some(crate::local_api::snapshot::LocalApiAsyncToolBackpressure {
            abandoned_request_count: crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
            saturation_threshold: crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
            saturated: true,
            oldest_tool: "background_shell_start".to_string(),
            oldest_summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            oldest_elapsed_before_timeout_seconds: 75,
            oldest_hard_timeout_seconds: 30,
            oldest_elapsed_seconds: 30,
        });
    current.async_tool_workers = vec![
        crate::local_api::snapshot::LocalApiAsyncToolWorker {
            request_id: "7".to_string(),
            lifecycle_state: "running".to_string(),
            thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
            owner: "wrapper_background_shell".to_string(),
            source_call_id: Some("call_1".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: Some(
                "wrapper_background_shell_terminal_without_tool_response".to_string(),
            ),
            observed_background_shell_job: Some(
                crate::local_api::snapshot::LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "failed".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 3,
                    recent_lines: vec!["boom".to_string()],
                },
            ),
            next_check_in_seconds: Some(30),
            runtime_elapsed_seconds: 75,
            state_elapsed_seconds: 75,
            hard_timeout_seconds: 30,
            supervision_classification: Some("tool_wedged".to_string()),
        },
        crate::local_api::snapshot::LocalApiAsyncToolWorker {
            request_id: "8".to_string(),
            lifecycle_state: "abandoned_after_timeout".to_string(),
            thread_name: "codexw-bgtool-background_shell_start-8".to_string(),
            owner: "wrapper_background_shell".to_string(),
            source_call_id: None,
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: None,
            observed_background_shell_job: None,
            next_check_in_seconds: None,
            runtime_elapsed_seconds: 30,
            state_elapsed_seconds: 30,
            hard_timeout_seconds: 30,
            supervision_classification: None,
        },
    ];
    current.supervision_notice = Some(crate::local_api::snapshot::LocalApiSupervisionNotice {
        classification: "tool_wedged".to_string(),
        recommended_action: "interrupt_or_exit_resume".to_string(),
        recovery_policy: crate::local_api::snapshot::LocalApiRecoveryPolicy {
            kind: "operator_interrupt_or_exit_resume".to_string(),
            automation_ready: false,
        },
        recovery_options: vec![
            crate::local_api::snapshot::LocalApiRecoveryOption {
                kind: "interrupt_turn".to_string(),
                label: "Interrupt the active turn".to_string(),
                automation_ready: false,
                cli_command: None,
                local_api_method: Some("POST".to_string()),
                local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
            },
            crate::local_api::snapshot::LocalApiRecoveryOption {
                kind: "exit_and_resume".to_string(),
                label: "Exit and resume the thread in a newer client".to_string(),
                automation_ready: false,
                cli_command: Some("codexw --cwd /tmp/repo resume thread_123".to_string()),
                local_api_method: None,
                local_api_path: None,
            },
        ],
        tool: "background_shell_start".to_string(),
        summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    });
    let log = new_event_log();

    publish_snapshot_change_events(&log, Some(&previous), &current);

    let events = events_since(&log, "sess_test", None);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event, "session.updated");
    assert_eq!(
        events[0].data["session"]["async_tool_supervision"]["classification"],
        "tool_wedged"
    );
    assert_eq!(
        events[0].data["session"]["async_tool_supervision"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recovery_policy"]["kind"],
        "operator_interrupt_or_exit_resume"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recovery_options"][1]["kind"],
        "exit_and_resume"
    );
    assert_eq!(events[1].event, "status.updated");
    assert_eq!(
        events[1].data["async_tool_supervision"]["classification"],
        "tool_wedged"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["observation_state"],
        "wrapper_background_shell_terminal_without_tool_response"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["observed_background_shell_job"]["status"],
        "failed"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["next_check_in_seconds"],
        30
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["supervision_classification"],
        "tool_wedged"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["observation_state"],
        "wrapper_background_shell_terminal_without_tool_response"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][1]["lifecycle_state"],
        "abandoned_after_timeout"
    );
    assert_eq!(events[1].data["async_tool_backpressure"]["saturated"], true);
    assert_eq!(
        events[1].data["supervision_notice"]["classification"],
        "tool_wedged"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["recovery_policy"]["automation_ready"],
        false
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["recovery_options"][0]["local_api_path"],
        "/api/v1/session/sess_test/turn/interrupt"
    );
}

#[test]
fn publish_client_event_emits_replayable_client_event() {
    let log = new_event_log();
    publish_client_event(
        &log,
        "sess_test",
        Some("fixture-events"),
        "selection.changed",
        serde_json::json!({
            "selection": "services",
        }),
    );

    let events = events_since(&log, "sess_test", None);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event, "client.event");
    assert_eq!(events[0].data["client_id"], "fixture-events");
    assert_eq!(events[0].data["event"], "selection.changed");
    assert_eq!(events[0].data["data"]["selection"], "services");
}

#[test]
fn client_event_route_publishes_replayable_semantic_event() {
    let snapshot = sample_snapshot();
    let queue = new_command_queue();
    let log = new_event_log();

    let response = super::super::routes::route_request_with_manager_and_events(
        &post_json_request(
            "/api/v1/session/sess_test/client_event",
            serde_json::json!({
                "client_id": "client_web",
                "event": "selection.changed",
                "data": {
                    "selection": "services"
                }
            }),
        ),
        &snapshot,
        &queue,
        &BackgroundShellManager::default(),
        &log,
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string()
        )]
    );
    let body = json_body(&response.body);
    assert_eq!(body["local_api_version"], CODEXW_LOCAL_API_VERSION);
    assert_eq!(body["event"], "selection.changed");
    assert_eq!(body["client_id"], "client_web");

    let events = events_since(&log, "sess_test", None);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event, "client.event");
    assert_eq!(events[0].data["event"], "selection.changed");
    assert_eq!(events[0].data["data"]["selection"], "services");

    let top_level = route_request(
        &post_json_request(
            "/api/v1/session/client_event",
            serde_json::json!({
                "session_id": "sess_test",
                "client_id": "client_web",
                "event": "selection.changed"
            }),
        ),
        &snapshot,
        &queue,
        None,
    );
    assert_eq!(top_level.status, 200);
    assert_eq!(
        top_level.headers,
        vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string()
        )]
    );
    let body = json_body(&top_level.body);
    assert_eq!(body["local_api_version"], CODEXW_LOCAL_API_VERSION);
    assert_eq!(body["event"], "selection.changed");
}
