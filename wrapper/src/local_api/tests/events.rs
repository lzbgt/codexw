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
    assert_eq!(events.len(), 6);
    assert_eq!(events[0].event, "session.updated");
    assert_eq!(
        events[0].data["session"]["attachment"]["id"],
        "attach:sess_test"
    );
    assert_eq!(events[0].data["attachment"]["scope"], "process");
    assert_eq!(events[0].data["attachment"]["client_id"], "client_web");
    assert_eq!(events[0].data["attachment"]["lease_seconds"], 300);
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

    drop(stream);
    handle.shutdown().expect("shutdown local api");
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
