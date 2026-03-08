use crate::session_realtime_item::render_realtime_item;
use crate::transcript_render::build_tool_user_input_response;
use crate::transcript_render::render_reasoning_item;
use crate::transcript_summary::summarize_terminal_interaction;
use serde_json::json;

#[test]
fn tool_user_input_defaults_to_first_option() {
    let response = build_tool_user_input_response(&json!({
        "questions": [
            {
                "id": "confirm_path",
                "options": [
                    {"label": "yes", "description": "Proceed"},
                    {"label": "no", "description": "Stop"}
                ]
            }
        ]
    }));
    assert_eq!(
        response,
        json!({
            "answers": {
                "confirm_path": { "answers": ["yes"] }
            }
        })
    );
}

#[test]
fn reasoning_prefers_summary_blocks() {
    let rendered = render_reasoning_item(&json!({
        "summary": ["First block", "Second block"],
        "content": ["raw detail"]
    }));
    assert_eq!(rendered, "First block\n\nSecond block");
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
