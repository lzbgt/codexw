use crate::transcript_plan_render::build_tool_user_input_response;
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
