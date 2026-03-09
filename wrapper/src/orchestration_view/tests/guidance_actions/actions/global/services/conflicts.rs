use super::super::super::super::*;

#[test]
fn actions_filter_renders_suggested_commands_for_conflicted_services() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first provider");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second provider");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps capabilities @api.http"));
    assert!(rendered.contains(":ps provide bg-1 <@other.role|none>"));
    assert!(rendered.contains(":clean services @api.http"));
    assert!(rendered.contains(":ps services @api.http"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(
        tool_rendered
            .contains("background_shell_inspect_capability {\"capability\":\"@api.http\"}")
    );
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@other.role\"]}"
    ));
    assert!(
        tool_rendered
            .contains("background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":null}")
    );
    assert!(
        tool_rendered.contains(
            "background_shell_clean {\"scope\":\"services\",\"capability\":\"@api.http\"}"
        )
    );

    let _ = services.background_shells.terminate_all_running();
}
