use super::super::super::*;

#[test]
fn status_overview_reports_orchestration_breakdown() {
    let mut state = crate::state::AppState::new(true, false);
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["agent-1".to_string()],
            prompt: Some("inspect auth flow".to_string()),
            agent_statuses: std::collections::BTreeMap::from([(
                "agent-1".to_string(),
                "running".to_string(),
            )]),
        },
    );
    state.cached_agent_threads = vec![
        CachedAgentThreadSummary {
            id: "agent-1".to_string(),
            status: "active".to_string(),
            preview: "inspect auth flow".to_string(),
            updated_at: Some(100),
        },
        CachedAgentThreadSummary {
            id: "agent-2".to_string(),
            status: "idle".to_string(),
            preview: "review API schema".to_string(),
            updated_at: Some(90),
        },
    ];
    state.background_terminals.insert(
        "proc-1".to_string(),
        crate::background_terminals::BackgroundTerminalSummary {
            item_id: "cmd-1".to_string(),
            process_id: "proc-1".to_string(),
            command_display: "python worker.py".to_string(),
            waiting: true,
            recent_inputs: Vec::new(),
            recent_output: vec!["ready".to_string()],
        },
    );
    state
        .background_shells
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start background shell");

    let rendered = render_status_overview(&test_cli(), "/tmp/project", &state).join("\n");
    assert!(rendered.contains(
        "orchestration   main=1 deps_blocking=0 deps_sidecar=2 waits=0 sidecar_agents=1 exec_prereqs=0 exec_sidecars=1 exec_services=0 services_ready=0 services_booting=0 services_untracked=0 services_conflicted=0 service_caps=0 service_cap_conflicts=0 cap_deps_missing=0 cap_deps_booting=0 cap_deps_ambiguous=0 agents_live=1 agents_cached=2"
    ));
    assert!(rendered.contains("active=1"));
    assert!(rendered.contains("idle=1"));
    assert!(rendered.contains("bg_shells=1"));
    assert!(rendered.contains("thread_terms=1"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn status_runtime_reports_background_classes() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite shell");
    state
        .background_shells
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start observation shell");
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service shell");
    state.background_terminals.insert(
        "proc-1".to_string(),
        crate::background_terminals::BackgroundTerminalSummary {
            item_id: "cmd-1".to_string(),
            process_id: "proc-1".to_string(),
            command_display: "python worker.py".to_string(),
            waiting: true,
            recent_inputs: Vec::new(),
            recent_output: vec!["ready".to_string()],
        },
    );

    let rendered = render_status_runtime(&cli, &state).join("\n");
    assert!(rendered.contains("background      4"));
    assert!(rendered.contains(
        "background cls  prereqs=1 shell_sidecars=1 services=1 services_ready=0 services_booting=0 services_untracked=1 services_conflicted=0 cap_deps_missing=0 cap_deps_booting=0 cap_deps_ambiguous=0 terminals=1"
    ));
    assert!(rendered.contains(
        "next action     Run `:ps blockers` to inspect the gating shell or wait dependency."
    ));
    let _ = state.background_shells.terminate_all_running();
}
