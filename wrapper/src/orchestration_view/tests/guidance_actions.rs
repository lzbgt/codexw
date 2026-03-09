use super::*;

#[test]
fn orchestration_service_counts_distinguish_ready_booting_and_untracked() {
    let state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": if cfg!(windows) {
                    "echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "printf 'READY\\n'; sleep 0.4"
                },
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready service");
    state
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": "sleep 0.4",
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");
    state
        .background_shells
        .start_from_tool(
            &serde_json::json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start untracked service");

    for _ in 0..40 {
        let summary = orchestration_background_summary(&state).expect("background summary");
        if summary.contains("services_ready=1") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let summary = orchestration_overview_summary(&state);
    assert!(summary.contains("exec_services=3"));
    assert!(summary.contains("services_ready=1"));
    assert!(summary.contains("services_booting=1"));
    assert!(summary.contains("services_untracked=1"));
    assert!(summary.contains("services_conflicted=0"));
    assert!(summary.contains("service_caps=0"));
    assert!(summary.contains("service_cap_conflicts=0"));

    let suffix = orchestration_prompt_suffix(&state).expect("prompt suffix");
    assert!(suffix.contains("1 service ready"));
    assert!(suffix.contains("1 service booting"));
    assert!(suffix.contains("1 service untracked"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn orchestration_guidance_prefers_blockers_then_ready_services_then_sidecars() {
    let blocked = crate::state::AppState::new(true, false);
    blocked
        .background_shells
        .start_from_tool(
            &serde_json::json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite shell");
    let blocked_hint = orchestration_guidance_summary(&blocked).expect("blocked guidance");
    assert!(blocked_hint.contains("blocked on 1 prerequisite shell"));
    let blocked_view = render_orchestration_guidance(&blocked);
    assert!(blocked_view.contains("Inspect :ps blockers"));
    let _ = blocked.background_shells.terminate_all_running();

    let ready = crate::state::AppState::new(true, false);
    ready
        .background_shells
        .start_from_tool(
            &serde_json::json!({
                "command": if cfg!(windows) {
                    "echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "printf 'READY\\n'; sleep 0.4"
                },
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready service");
    for _ in 0..40 {
        if let Some(hint) = orchestration_guidance_summary(&ready)
            && hint.contains("ready for reuse")
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let ready_hint = orchestration_guidance_summary(&ready).expect("ready guidance");
    assert!(ready_hint.contains("ready for reuse"));
    let _ = ready.background_shells.terminate_all_running();

    let mut sidecar = crate::state::AppState::new(true, false);
    sidecar.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::new(),
        },
    );
    let sidecar_hint = orchestration_guidance_summary(&sidecar).expect("sidecar guidance");
    assert!(sidecar_hint.contains("running without blocking"));
}

#[test]
fn orchestration_guidance_surfaces_service_capability_conflicts_before_ready_reuse() {
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
    let hint = orchestration_guidance_summary(&services).expect("conflict guidance");
    assert!(hint.contains("capability conflict"));
    let rendered = render_orchestration_guidance(&services);
    assert!(rendered.contains("@api.http"));
    assert!(rendered.contains(":ps capabilities"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn orchestration_guidance_prioritizes_missing_blocking_service_dependencies() {
    let blocked = crate::state::AppState::new(true, false);
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
        .expect("start dependent shell");
    let hint = orchestration_guidance_summary(&blocked).expect("dependency guidance");
    assert!(hint.contains("missing service capability @api.http"));
    let rendered = render_orchestration_guidance(&blocked);
    assert!(rendered.contains(":ps capabilities"));
    assert!(rendered.contains(":ps dependencies missing @api.http"));
    let blockers = render_orchestration_workers_with_filter(&blocked, WorkerFilter::Blockers);
    assert!(
        blockers.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        )
    );
    let _ = blocked.background_shells.terminate_all_running();
}

#[test]
fn guidance_filter_renders_next_action_section() {
    let mut state = crate::state::AppState::new(true, false);
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "wait".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: None,
            agent_statuses: BTreeMap::new(),
        },
    );
    let rendered = render_orchestration_workers_with_filter(&state, WorkerFilter::Guidance);
    assert!(rendered.contains("Next action:"));
    assert!(rendered.contains("blocked on 1 agent wait"));
    assert!(rendered.contains(":multi-agents"));
}

#[test]
fn guidance_filter_surfaces_contract_fix_for_untracked_services() {
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

    let rendered = render_orchestration_guidance(&services);
    assert!(rendered.contains("missing readiness or attachment metadata"));
    assert!(rendered.contains(":ps services untracked"));
    assert!(rendered.contains(":ps contract bg-1 <json-object>"));
    assert!(rendered.contains(":ps relabel bg-1 <label|none>"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn guidance_filter_uses_concrete_wait_for_single_booting_service() {
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

    let rendered = render_orchestration_guidance(&services);
    assert!(rendered.contains("still booting"));
    assert!(rendered.contains(":ps wait bg-1 [timeoutMs]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn guidance_filter_uses_concrete_provider_ref_for_single_ready_service() {
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

    let rendered = render_orchestration_guidance(&services);
    assert!(rendered.contains(":ps attach bg-1"));
    assert!(rendered.contains(":ps run bg-1 health"));
    assert!(!rendered.contains(":ps run bg-1 health [json-args]"));
    let _ = services.background_shells.terminate_all_running();
}

#[test]
fn guidance_filter_uses_concrete_wait_for_booting_blocker_provider() {
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

    let rendered = render_orchestration_guidance(&services);
    assert!(rendered.contains(":ps services booting @api.http"));
    assert!(rendered.contains(":ps wait bg-1 [timeoutMs]"));
    let _ = services.background_shells.terminate_all_running();
}

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
