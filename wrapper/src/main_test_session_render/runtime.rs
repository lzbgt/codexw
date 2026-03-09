use crate::background_shells::BackgroundShellManager;
use crate::client_dynamic_tools::execute_dynamic_tool_call;
use crate::session_realtime_item::render_realtime_item;
use crate::transcript_approval_summary::summarize_terminal_interaction;
use serde_json::json;

#[test]
fn unknown_dynamic_tool_reports_failure_to_model() {
    let response = execute_dynamic_tool_call(
        &json!({
            "tool": "lookup_ticket",
            "arguments": {"id": "ABC-123"}
        }),
        "/tmp",
        &BackgroundShellManager::default(),
    );
    assert_eq!(response["success"], false);
    assert_eq!(
        response["contentItems"][0]["text"],
        "unsupported client dynamic tool `lookup_ticket`"
    );
}

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
