use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use crate::background_shells::BackgroundShellManager;

use super::events_since;
use super::get_request;
use super::json_body;
use super::new_command_queue;
use super::new_event_log;
use super::publish_snapshot_change_events;
use super::route_request;
use super::sample_snapshot;
use super::start_local_api;

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
    assert!(response_text.contains("event: session.updated"));
    assert!(response_text.contains("event: turn.updated"));

    drop(stream);
    handle.shutdown().expect("shutdown local api");
}
