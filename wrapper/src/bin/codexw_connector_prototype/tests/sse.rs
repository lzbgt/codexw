use serde_json::Value;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;

use super::super::sse::complete_sse_lines;
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
    assert_eq!(
        json["broker"]["adapter_version"],
        CODEXW_BROKER_ADAPTER_VERSION
    );
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

#[test]
fn complete_sse_lines_preserves_fragmented_first_data_line_until_newline_arrives() {
    let mut pending = String::new();

    let first = complete_sse_lines("data: {\"async_tool_back", &mut pending);
    let second = complete_sse_lines(
        "pressure\":{\"abandoned_request_count\":1}}\n\n",
        &mut pending,
    );

    assert!(first.is_empty());
    assert_eq!(
        second,
        vec![
            "data: {\"async_tool_backpressure\":{\"abandoned_request_count\":1}}".to_string(),
            "".to_string()
        ]
    );
    assert!(pending.is_empty());
}

#[test]
fn complete_sse_lines_handles_multiple_crlf_terminated_lines_per_chunk() {
    let mut pending = String::new();

    let completed = complete_sse_lines("id: 30\r\nevent: status.updated\r\n", &mut pending);

    assert_eq!(
        completed,
        vec!["id: 30".to_string(), "event: status.updated".to_string()]
    );
    assert!(pending.is_empty());
}
