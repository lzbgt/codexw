use super::*;

#[test]
fn focused_actions_for_untracked_capability_render_contract_fixes() {
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
        .expect("start untracked service");

    let rendered = render_orchestration_actions_for_capability(&services, "@api.http")
        .expect("focused operator actions");
    assert!(rendered.contains("Suggested actions (@api.http):"));
    assert!(rendered.contains(":ps services untracked @api.http"));
    assert!(rendered.contains(":ps contract bg-1 <json-object>"));
    assert!(rendered.contains(":ps relabel bg-1 <label|none>"));

    let tool_rendered = render_orchestration_actions_for_tool_capability(&services, "@api.http")
        .expect("focused tool actions");
    assert!(tool_rendered.contains("Suggested actions (@api.http):"));
    assert!(tool_rendered.contains(
        "background_shell_list_services {\"status\":\"untracked\",\"capability\":\"@api.http\"}"
    ));
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"label\":\"service-label\"}"
    ));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn focused_guidance_and_actions_can_target_one_capability() {
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
        .expect("start service");

    let guidance = render_orchestration_guidance_for_capability(&services, "@api.http")
        .expect("focused guidance");
    assert!(guidance.contains("Next action (@api.http):"));
    assert!(guidance.contains("untracked service"));
    assert!(guidance.contains(":ps services untracked @api.http"));
    assert!(guidance.contains(":ps contract bg-1 <json-object>"));

    let operator_actions = render_orchestration_actions_for_capability(&services, "@api.http")
        .expect("focused operator actions");
    assert!(operator_actions.contains("Suggested actions (@api.http):"));
    assert!(operator_actions.contains(":ps services untracked @api.http"));
    assert!(operator_actions.contains(":ps contract bg-1 <json-object>"));

    let tool_actions = render_orchestration_actions_for_tool_capability(&services, "@api.http")
        .expect("focused tool actions");
    assert!(tool_actions.contains("Suggested actions (@api.http):"));
    assert!(tool_actions.contains(
        "background_shell_list_services {\"status\":\"untracked\",\"capability\":\"@api.http\"}"
    ));
    assert!(tool_actions.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn focused_booting_capability_actions_use_concrete_provider_ref() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");

    let guidance = render_orchestration_guidance_for_capability(&services, "@api.http")
        .expect("focused guidance");
    assert!(guidance.contains("booting"));
    assert!(guidance.contains(":ps wait bg-1 5000"));

    let operator_actions = render_orchestration_actions_for_capability(&services, "@api.http")
        .expect("focused operator actions");
    assert!(operator_actions.contains(":ps wait bg-1 5000"));

    let tool_actions = render_orchestration_actions_for_tool_capability(&services, "@api.http")
        .expect("focused tool actions");
    assert!(
        tool_actions
            .contains("background_shell_wait_ready {\"jobId\":\"bg-1\",\"timeoutMs\":5000}")
    );
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn focused_ready_capability_actions_use_concrete_provider_ref() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "readyPattern": "READY",
                "recipes": [{
                    "name": "health",
                    "action": {
                        "type": "stdin",
                        "text": "health"
                    }
                }]
            }),
            "/tmp",
        )
        .expect("start ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait for ready service");

    let guidance = render_orchestration_guidance_for_capability(&services, "@api.http")
        .expect("focused guidance");
    assert!(guidance.contains(":ps attach bg-1"));
    assert!(guidance.contains(":ps run bg-1 health"));
    assert!(!guidance.contains(":ps run bg-1 health [json-args]"));

    let tool_actions = render_orchestration_actions_for_tool_capability(&services, "@api.http")
        .expect("focused tool actions");
    assert!(tool_actions.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(
        tool_actions
            .contains("background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"health\"}")
    );
    let _ = services.background_shells.terminate_all_running();
}

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

#[test]
fn focused_ambiguous_capability_actions_recommend_non_conflicting_fix() {
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

    let operator_actions = render_orchestration_actions_for_capability(&services, "@api.http")
        .expect("focused operator actions");
    assert!(operator_actions.contains(":ps provide bg-1 <@other.role|none>"));
    assert!(!operator_actions.contains(":ps provide <jobId|alias|n> <@capability...|none>"));

    let tool_actions = render_orchestration_actions_for_tool_capability(&services, "@api.http")
        .expect("focused tool actions");
    assert!(tool_actions.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@other.role\"]}"
    ));
    assert!(
        tool_actions
            .contains("background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":null}")
    );
    assert!(!tool_actions.contains(
        "background_shell_update_service {\"jobId\":\"<jobId|alias|n>\",\"capabilities\":[\"@api.http\"]}"
    ));
    let _ = services.background_shells.terminate_all_running();
}
