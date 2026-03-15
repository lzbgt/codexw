use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use super::*;
use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;
use crate::background_shells::BackgroundShellManager;

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
        events[0].data["session"]["supervision_notice"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["source_call_id"],
        "call_1"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["target_background_shell_job_id"],
        "bg-1"
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
    assert_eq!(events[2].data["async_tool_supervision"]["request_id"], "7");
    assert_eq!(
        events[2].data["async_tool_supervision"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_supervision"]["observed_background_shell_job"]["last_output_age_seconds"],
        2
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
        events[2].data["async_tool_backpressure"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["recovery_policy"]["kind"],
        "warn_only"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["recovery_policy"]["automation_ready"],
        false
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["recovery_options"][0]["kind"],
        "observe_status"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["recovery_options"][1]["local_api_path"],
        "/api/v1/session/sess_test/turn/interrupt"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["recovery_options"][2]["kind"],
        "exit_and_resume"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_request_id"],
        "8"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_thread_name"],
        "codexw-bgtool-background_shell_start-8"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_hard_timeout_seconds"],
        15
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_source_call_id"],
        "call_2"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        events[2].data["async_tool_backpressure"]["oldest_observed_background_shell_job"]["job_id"],
        "bg-1"
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
        events[2].data["async_tool_workers"][0]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][0]["observed_background_shell_job"]["last_output_age_seconds"],
        2
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
        events[2].data["async_tool_workers"][1]["source_call_id"],
        "call_2"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][1]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][1]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][1]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][1]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        events[2].data["async_tool_workers"][1]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["recommended_action"],
        "observe_or_interrupt"
    );
    assert_eq!(events[2].data["supervision_notice"]["request_id"], "7");
    assert_eq!(
        events[2].data["supervision_notice"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["source_call_id"],
        "call_1"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["observation_state"],
        "wrapper_background_shell_streaming_output"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["output_state"],
        "recent_output_observed"
    );
    assert_eq!(
        events[2].data["supervision_notice"]["observed_background_shell_job"]["job_id"],
        "bg-1"
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
