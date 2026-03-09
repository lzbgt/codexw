use super::super::super::*;

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
