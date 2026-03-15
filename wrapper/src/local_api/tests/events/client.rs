use super::*;
use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;
use crate::background_shells::BackgroundShellManager;
use crate::local_api::publish_client_event;

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

    let response = super::super::super::routes::route_request_with_manager_and_events(
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
