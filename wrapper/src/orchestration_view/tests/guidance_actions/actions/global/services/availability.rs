use super::super::super::super::*;

#[test]
fn actions_filter_renders_contract_suggestions_for_untracked_services() {
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

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps services untracked"));
    assert!(rendered.contains(":ps contract bg-1 <json-object>"));
    assert!(rendered.contains(":ps relabel bg-1 <label|none>"));

    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_wait_for_single_booting_service() {
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

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps services booting"));
    assert!(rendered.contains(":ps wait bg-1 [timeoutMs]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_provider_ref_for_single_ready_service() {
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
        .expect("wait ready service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps attach bg-1"));
    assert!(rendered.contains(":ps run bg-1 health"));
    assert!(!rendered.contains(":ps run bg-1 health [json-args]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn ready_service_guidance_omits_invoke_step_when_only_descriptive_recipes_exist() {
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
                    "name": "docs",
                    "description": "Read usage notes"
                }]
            }),
            "/tmp",
        )
        .expect("start ready service with descriptive recipe");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait ready service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps attach bg-1"));
    assert!(!rendered.contains(":ps run bg-1"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(!tool_rendered.contains("background_shell_invoke_recipe"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn ready_service_guidance_prefers_health_recipe_over_earlier_generic_recipe() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "readyPattern": "READY",
                "recipes": [
                    {
                        "name": "query",
                        "action": {
                            "type": "stdin",
                            "text": "query {{key}}"
                        },
                        "parameters": [{"name": "key", "required": true}]
                    },
                    {
                        "name": "health",
                        "action": {
                            "type": "stdin",
                            "text": "health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait ready service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps run bg-1 health"));
    assert!(!rendered.contains(":ps run bg-1 query"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn ready_service_guidance_includes_example_args_for_parameterized_recipe() {
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
                    "name": "query",
                    "action": {
                        "type": "stdin",
                        "text": "query {{key}} {{mode}}"
                    },
                    "parameters": [
                        {"name": "key", "required": true},
                        {"name": "mode", "default": "fast"}
                    ]
                }]
            }),
            "/tmp",
        )
        .expect("start ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait ready service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps run bg-1 query {\"key\":\"value\",\"mode\":\"fast\"}"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn global_ready_service_actions_use_explicit_reference_syntax_when_provider_is_not_unique() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
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
        .expect("start first ready service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
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
        .expect("start second ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait ready service one");
    services
        .background_shells
        .wait_ready_for_operator("bg-2", 1000)
        .expect("wait ready service two");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps attach <jobId|alias|@capability|n>"));
    assert!(rendered.contains(":ps run <jobId|alias|@capability|n> <recipe> [json-args]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn global_booting_service_actions_use_explicit_reference_syntax_when_provider_is_not_unique() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start first booting service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start second booting service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps wait <jobId|alias|@capability|n> [timeoutMs]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn global_untracked_service_actions_use_explicit_reference_syntax_when_provider_is_not_unique() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start first untracked service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start second untracked service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps contract <jobId|alias|@capability|n> <json-object>"));
    assert!(rendered.contains(":ps relabel <jobId|alias|@capability|n> <label|none>"));
    let _ = services.background_shells.terminate_all_running();
}
