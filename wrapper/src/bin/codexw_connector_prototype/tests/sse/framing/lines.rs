use crate::sse::complete_sse_lines;

#[test]
fn complete_sse_lines_preserves_fragmented_first_data_line_until_newline_arrives() {
    let mut pending = String::new();

    let first = complete_sse_lines(
        "data: {\"async_tool_backpressure\":{\"abandoned_request_count\":1,\"recommended_action\":\"observe_or_in",
        &mut pending,
    );
    let second = complete_sse_lines(
        "terrupt\",\"recovery_policy\":{\"kind\":\"warn_only\",\"automation_ready\":false},\"recovery_options\":[{\"kind\":\"observe_status\",\"local_api_path\":\"/sessions/sess_1/status\"}],\"oldest_request_id\":\"7\"}}\n\n",
        &mut pending,
    );

    assert!(first.is_empty());
    assert_eq!(
        second,
        vec![
            "data: {\"async_tool_backpressure\":{\"abandoned_request_count\":1,\"recommended_action\":\"observe_or_interrupt\",\"recovery_policy\":{\"kind\":\"warn_only\",\"automation_ready\":false},\"recovery_options\":[{\"kind\":\"observe_status\",\"local_api_path\":\"/sessions/sess_1/status\"}],\"oldest_request_id\":\"7\"}}".to_string(),
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
