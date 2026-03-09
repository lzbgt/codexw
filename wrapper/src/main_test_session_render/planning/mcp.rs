use crate::transcript_plan_render::build_mcp_elicitation_response;
use serde_json::json;

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
