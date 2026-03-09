use crate::background_shells::BackgroundShellInteractionAction;
use crate::background_shells::parse_background_shell_interaction_recipes;
use serde_json::json;

#[test]
fn recipe_parser_accepts_info_alias_and_structured_actions() {
    let recipes = parse_background_shell_interaction_recipes(Some(&json!([
        {
            "name": "docs",
            "description": "Read the runbook",
            "action": { "type": "info" }
        },
        {
            "name": "health",
            "parameters": [
                { "name": "id", "default": "health" }
            ],
            "action": {
                "type": "http",
                "method": "get",
                "path": "/{{id}}",
                "headers": { "Accept": "application/json" },
                "expectedStatus": 200
            }
        }
    ])))
    .expect("parse recipes");

    assert_eq!(recipes.len(), 2);
    assert!(matches!(
        recipes[0].action,
        BackgroundShellInteractionAction::Informational
    ));
    assert_eq!(recipes[1].parameters.len(), 1);
    assert_eq!(recipes[1].parameters[0].name, "id");
    assert_eq!(recipes[1].parameters[0].default.as_deref(), Some("health"));
    assert!(matches!(
        &recipes[1].action,
        BackgroundShellInteractionAction::Http {
            method,
            path,
            headers,
            expected_status,
            ..
        } if method == "GET"
            && path == "/{{id}}"
            && headers == &vec![("Accept".to_string(), "application/json".to_string())]
            && *expected_status == Some(200)
    ));
}

#[test]
fn recipe_parser_rejects_invalid_expected_status_range() {
    let err = parse_background_shell_interaction_recipes(Some(&json!([
        {
            "name": "health",
            "action": {
                "type": "http",
                "method": "GET",
                "path": "/health",
                "expectedStatus": 99
            }
        }
    ])))
    .expect_err("expected invalid status to fail");

    assert!(err.contains("expectedStatus` must be between 100 and 599"));
}

#[test]
fn recipe_parser_rejects_non_string_redis_command_entries() {
    let err = parse_background_shell_interaction_recipes(Some(&json!([
        {
            "name": "ping",
            "action": {
                "type": "redis",
                "command": ["PING", 1]
            }
        }
    ])))
    .expect_err("expected invalid redis command to fail");

    assert!(err.contains("action.command[1]` must be a string"));
}

#[test]
fn recipe_parser_rejects_blank_header_names() {
    let err = parse_background_shell_interaction_recipes(Some(&json!([
        {
            "name": "health",
            "action": {
                "type": "http",
                "method": "GET",
                "path": "/health",
                "headers": { " ": "application/json" }
            }
        }
    ])))
    .expect_err("expected invalid header name to fail");

    assert!(err.contains("action.headers` keys must be non-empty"));
}
