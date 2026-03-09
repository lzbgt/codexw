use super::super::super::*;

#[test]
fn focused_blockers_can_target_one_capability() {
    let blocked = crate::state::AppState::new(true, false);
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start retargetable service");
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api blocker");
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    let blockers =
        render_orchestration_blockers_for_capability(&blocked, "@api.http").expect("focus");
    assert!(blockers.contains("Dependencies (@api.http):"));
    assert!(blockers.contains("shell:bg-2 -> capability:@api.http"));
    assert!(!blockers.contains("db.redis"));

    let guidance = render_orchestration_guidance_for_capability(&blocked, "@api.http")
        .expect("focused guidance");
    assert!(guidance.contains(":ps provide bg-1 @api.http"));
    assert!(guidance.contains(":ps dependencies missing @api.http"));

    let operator_actions = render_orchestration_actions_for_capability(&blocked, "@api.http")
        .expect("focused operator actions");
    assert!(operator_actions.contains(":ps provide bg-1 @api.http"));
    assert!(operator_actions.contains(":ps depend bg-2 <@capability...|none>"));
    assert!(operator_actions.contains(":clean blockers @api.http"));

    let tool_actions = render_orchestration_actions_for_tool_capability(&blocked, "@api.http")
        .expect("focused tool actions");
    assert!(tool_actions.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@api.http\"]}"
    ));
    assert!(tool_actions.contains(
        "background_shell_update_dependencies {\"jobId\":\"bg-2\",\"dependsOnCapabilities\":[\"@other.role\"]}"
    ));
    assert!(
        tool_actions.contains(
            "background_shell_clean {\"scope\":\"blockers\",\"capability\":\"@api.http\"}"
        )
    );
    let _ = blocked.background_shells.terminate_all_running();
}
