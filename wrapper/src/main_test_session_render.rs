use crate::session_realtime_item::render_realtime_item;
use crate::transcript_approval_summary::summarize_terminal_interaction;
use crate::transcript_plan_render::build_dynamic_tool_call_response;
use crate::transcript_plan_render::build_mcp_elicitation_response;
use crate::transcript_plan_render::build_tool_user_input_response;
use crate::transcript_plan_render::render_reasoning_item;
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
fn mcp_form_elicitation_prefers_defaults_and_required_fallbacks() {
    let response = build_mcp_elicitation_response(&json!({
        "mode": "form",
        "requestedSchema": {
            "type": "object",
            "required": ["email", "count", "choices", "confirm"],
            "properties": {
                "email": {"type": "string", "format": "email"},
                "count": {"type": "integer", "minimum": 2},
                "choices": {
                    "type": "array",
                    "minItems": 1,
                    "items": {"enum": ["alpha", "beta"]}
                },
                "confirm": {"type": "boolean"},
                "optional_note": {"type": "string"},
                "preset": {"type": "string", "default": "keep-me"}
            }
        }
    }));
    assert_eq!(
        response,
        json!({
            "action": "accept",
            "content": {
                "email": "user@example.com",
                "count": 2,
                "choices": ["alpha"],
                "confirm": false,
                "preset": "keep-me"
            },
            "_meta": null
        })
    );
}

#[test]
fn mcp_url_elicitation_is_cancelled_for_unattended_mode() {
    let response = build_mcp_elicitation_response(&json!({
        "mode": "url",
        "message": "open browser",
        "url": "https://example.com/auth",
        "elicitationId": "eli-1"
    }));
    assert_eq!(
        response,
        json!({
            "action": "cancel",
            "content": null,
            "_meta": null
        })
    );
}

#[test]
fn dynamic_tool_fallback_names_tool_and_arguments() {
    let response = build_dynamic_tool_call_response(&json!({
        "tool": "lookup_ticket",
        "arguments": {"id": "ABC-123"}
    }));
    assert_eq!(
        response,
        json!({
            "contentItems": [
                {
                    "type": "inputText",
                    "text": "codexw cannot execute client-side dynamic tool `lookup_ticket` automatically; arguments={\"id\":\"ABC-123\"}"
                }
            ],
            "success": false
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
