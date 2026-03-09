use super::*;

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
        .expect("start first service");
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
        .expect("start second service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps capabilities @api.http"));
    assert!(rendered.contains(":ps provide bg-1 <@other.role|none>"));
    assert!(rendered.contains(":clean services @api.http"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
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

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps capabilities @api.http"));
    let _ = services.background_shells.terminate_all_running();
}

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

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
    assert!(tool_rendered.contains("background_shell_list_services {\"status\":\"untracked\"}"));
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"label\":\"service-label\"}"
    ));

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps services untracked"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_wait_for_booting_blocker_provider() {
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
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start blocked prerequisite");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps services booting @api.http"));
    assert!(rendered.contains(":ps wait bg-1 5000"));
    assert!(rendered.contains(":ps dependencies booting @api.http"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
    assert!(tool_rendered.contains(
        "background_shell_list_services {\"status\":\"booting\",\"capability\":\"@api.http\"}"
    ));
    assert!(
        tool_rendered
            .contains("background_shell_wait_ready {\"jobId\":\"bg-1\",\"timeoutMs\":5000}")
    );
    assert!(tool_rendered.contains(
        "orchestration_list_dependencies {\"filter\":\"booting\",\"capability\":\"@api.http\"}"
    ));

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps wait bg-1 5000"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_poll_for_single_generic_blocker() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite"
            }),
            "/tmp",
        )
        .expect("start generic blocker");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps blockers"));
    assert!(rendered.contains(":ps poll bg-1"));
    assert!(rendered.contains(":clean blockers"));
    assert!(!rendered.contains(":ps wait <job> [timeoutMs]"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
    assert!(tool_rendered.contains("orchestration_list_workers {\"filter\":\"blockers\"}"));
    assert!(tool_rendered.contains("background_shell_poll {\"jobId\":\"bg-1\"}"));
    assert!(tool_rendered.contains("background_shell_clean {\"scope\":\"blockers\"}"));
    assert!(!tool_rendered.contains("background_shell_wait_ready"));

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps poll bg-1"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_real_reference_placeholder_for_non_unique_generic_blockers() {
    let services = crate::state::AppState::new(true, false);
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite"
            }),
            "/tmp",
        )
        .expect("start first generic blocker");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite"
            }),
            "/tmp",
        )
        .expect("start second generic blocker");

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(
        tool_rendered.contains("background_shell_poll {\"jobId\":\"<jobId|alias|@capability>\"}")
    );
    assert!(!tool_rendered.contains("background_shell_poll {\"jobId\":\"bg-...\"}"));
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

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
    assert!(tool_rendered.contains("background_shell_list_services {\"status\":\"booting\"}"));
    assert!(
        tool_rendered
            .contains("background_shell_wait_ready {\"jobId\":\"bg-1\",\"timeoutMs\":5000}")
    );

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps wait bg-1 [timeoutMs]"));
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
        .expect("wait for ready service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps attach bg-1"));
    assert!(rendered.contains(":ps run bg-1 health"));
    assert!(!rendered.contains(":ps run bg-1 health [json-args]"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains("Suggested actions:"));
    assert!(tool_rendered.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(
        tool_rendered
            .contains("background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"health\"}")
    );

    let filtered = render_orchestration_workers_with_filter(&services, WorkerFilter::Actions);
    assert!(filtered.contains("Suggested actions:"));
    assert!(filtered.contains(":ps attach bg-1"));
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
                "recipes": [{"name": "docs", "description": "Read usage notes"}]
            }),
            "/tmp",
        )
        .expect("start ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait for ready service");

    let operator_guidance = render_orchestration_guidance(&services);
    assert!(operator_guidance.contains(":ps attach bg-1"));
    assert!(!operator_guidance.contains(":ps run bg-1"));

    let operator_actions = render_orchestration_actions(&services);
    assert!(operator_actions.contains(":ps attach bg-1"));
    assert!(!operator_actions.contains(":ps run bg-1"));

    let tool_guidance = render_orchestration_guidance_for_tool(&services);
    assert!(tool_guidance.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(!tool_guidance.contains("background_shell_invoke_recipe"));

    let tool_actions = render_orchestration_actions_for_tool(&services);
    assert!(tool_actions.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(!tool_actions.contains("background_shell_invoke_recipe"));

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
        .expect("wait for ready service");

    let operator_actions = render_orchestration_actions(&services);
    assert!(operator_actions.contains(":ps run bg-1 health"));
    assert!(!operator_actions.contains(":ps run bg-1 query"));

    let tool_actions = render_orchestration_actions_for_tool(&services);
    assert!(
        tool_actions
            .contains("background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"health\"}")
    );
    assert!(!tool_actions.contains("\"recipe\":\"query\""));
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
        .expect("wait for ready service");

    let operator_actions = render_orchestration_actions(&services);
    assert!(operator_actions.contains(":ps run bg-1 query {\"key\":\"value\",\"mode\":\"fast\"}"));
    assert!(!operator_actions.contains(":ps run bg-1 query [json-args]"));

    let tool_actions = render_orchestration_actions_for_tool(&services);
    assert!(tool_actions.contains(
        "background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"query\",\"args\":{\"key\":\"value\",\"mode\":\"fast\"}}"
    ));
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
                "capabilities": ["api.http"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start first ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait for first ready service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
                "capabilities": ["db.redis"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start second ready service");
    services
        .background_shells
        .wait_ready_for_operator("bg-2", 1000)
        .expect("wait for second ready service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps attach <jobId|alias|@capability|n>"));
    assert!(rendered.contains(":ps run <jobId|alias|@capability|n> <recipe> [json-args]"));
    assert!(!rendered.contains(":ps attach <job|@capability>"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(
        tool_rendered.contains("background_shell_attach {\"jobId\":\"<jobId|alias|@capability>\"}")
    );
    assert!(tool_rendered.contains(
        "background_shell_invoke_recipe {\"jobId\":\"<jobId|alias|@capability>\",\"recipe\":\"...\"}"
    ));
    assert!(!tool_rendered.contains("background_shell_attach {\"jobId\":\"@capability\"}"));
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
                "capabilities": ["api.http"],
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
                "capabilities": ["db.redis"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start second booting service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps wait <jobId|alias|@capability|n> [timeoutMs]"));
    assert!(!rendered.contains(":ps wait <job|@capability> [timeoutMs]"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains(
        "background_shell_wait_ready {\"jobId\":\"<jobId|alias|@capability>\",\"timeoutMs\":5000}"
    ));
    assert!(
        !tool_rendered
            .contains("background_shell_wait_ready {\"jobId\":\"@capability\",\"timeoutMs\":5000}")
    );
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
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first untracked service");
    services
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start second untracked service");

    let rendered = render_orchestration_actions(&services);
    assert!(rendered.contains(":ps contract <jobId|alias|@capability|n> <json-object>"));
    assert!(rendered.contains(":ps relabel <jobId|alias|@capability|n> <label|none>"));

    let tool_rendered = render_orchestration_actions_for_tool(&services);
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"label\":\"service-label\"}"
    ));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn actions_filter_uses_concrete_provider_ref_for_missing_capability_when_unique_service_exists() {
    let state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start retargetable service");
    state
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start missing blocker");

    let rendered = render_orchestration_actions(&state);
    assert!(rendered.contains("Suggested actions:"));
    assert!(rendered.contains(":ps provide bg-1 @api.http"));
    assert!(rendered.contains(":ps depend bg-2 <@capability...|none>"));

    let tool_rendered = render_orchestration_actions_for_tool(&state);
    assert!(tool_rendered.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@api.http\"]}"
    ));
    assert!(tool_rendered.contains(
        "background_shell_update_dependencies {\"jobId\":\"bg-2\",\"dependsOnCapabilities\":[\"@other.role\"]}"
    ));
    let _ = state.background_shells.terminate_all_running();
}
