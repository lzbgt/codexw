use crate::session_realtime_item::render_realtime_item;
use crate::transcript_approval_summary::summarize_terminal_interaction;
use crate::transcript_approval_summary::summarize_tool_request;
use crate::transcript_item_summary::summarize_tool_item;
use serde_json::json;

#[test]
fn empty_terminal_interaction_is_suppressed() {
    assert_eq!(
        summarize_terminal_interaction(&json!({
            "processId": "123",
            "stdin": ""
        })),
        None
    );
}

#[test]
fn terminal_interaction_only_surfaces_meaningful_stdin() {
    assert_eq!(
        summarize_terminal_interaction(&json!({
            "processId": "123",
            "stdin": "yes\n"
        })),
        Some("process=123 stdin=yes".to_string())
    );
}

#[test]
fn realtime_item_prefers_text_content() {
    let rendered = render_realtime_item(&json!({
        "type": "message",
        "id": "msg-1",
        "role": "assistant",
        "content": [
            {"text": "first line"},
            {"transcript": "second line"}
        ]
    }));
    assert!(rendered.contains("type            message"));
    assert!(rendered.contains("id              msg-1"));
    assert!(rendered.contains("role            assistant"));
    assert!(rendered.contains("first line"));
    assert!(rendered.contains("second line"));
}

#[test]
fn tool_request_summary_includes_tool_and_arguments() {
    let summary = summarize_tool_request(&json!({
        "tool": "lookup_ticket",
        "arguments": {
            "id": "ABC-123",
            "project": "infra"
        }
    }));

    assert!(summary.contains("lookup_ticket"));
    assert!(summary.contains("ABC-123"));
    assert!(summary.contains("infra"));
}

#[test]
fn dynamic_tool_item_summary_uses_generic_result_rendering() {
    let summary = summarize_tool_item(
        "mcpToolCall",
        &json!({
            "tool": "lookup_ticket",
            "contentItems": [{"text": "ticket ABC-123 is open"}]
        }),
        false,
    );

    assert!(summary.contains("lookup_ticket"));
    assert!(summary.contains("ticket ABC-123 is open"));
}
