use serde_json::Value;

use super::super::sse::wrap_event_payload;

#[test]
fn wrap_event_payload_preserves_json_and_adds_broker_metadata() {
    let wrapped = wrap_event_payload(
        vec![r#"{"session_id":"sess_1","value":1}"#.to_string()],
        "codexw-lab",
        "mac-mini-01",
    );
    let json: Value = serde_json::from_str(&wrapped).expect("valid json");
    assert_eq!(json["source"], "codexw");
    assert_eq!(json["broker"]["agent_id"], "codexw-lab");
    assert_eq!(json["broker"]["deployment_id"], "mac-mini-01");
    assert_eq!(json["data"]["session_id"], "sess_1");
    assert_eq!(json["data"]["value"], 1);
}

#[test]
fn wrap_event_payload_falls_back_to_string_for_non_json_data() {
    let wrapped = wrap_event_payload(
        vec!["plain text update".to_string()],
        "codexw-lab",
        "mac-mini-01",
    );
    let json: Value = serde_json::from_str(&wrapped).expect("valid json");
    assert_eq!(json["data"], "plain text update");
}
