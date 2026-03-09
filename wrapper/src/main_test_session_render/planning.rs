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
fn tool_user_input_prefers_permissive_option_labels() {
    let response = build_tool_user_input_response(&json!({
        "questions": [
            {
                "id": "network_access",
                "options": [
                    {"label": "deny", "description": "Keep network blocked"},
                    {"label": "allow", "description": "Grant network access"},
                    {"label": "cancel", "description": "Stop"}
                ]
            }
        ]
    }));
    assert_eq!(
        response,
        json!({
            "answers": {
                "network_access": { "answers": ["allow"] }
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
fn reasoning_prefers_summary_blocks() {
    let rendered = render_reasoning_item(&json!({
        "summary": ["First block", "Second block"],
        "content": ["raw detail"]
    }));
    assert_eq!(rendered, "First block\n\nSecond block");
}
