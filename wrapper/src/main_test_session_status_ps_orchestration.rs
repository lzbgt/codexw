use super::*;

#[test]
fn ps_filter_parser_accepts_worker_class_aliases() {
    assert_eq!(parse_ps_filter(None), Some(WorkerFilter::All));
    assert_eq!(parse_ps_filter(Some("all")), Some(WorkerFilter::All));
    assert_eq!(
        parse_ps_filter(Some("guidance")),
        Some(WorkerFilter::Guidance)
    );
    assert_eq!(parse_ps_filter(Some("next")), Some(WorkerFilter::Guidance));
    assert_eq!(
        parse_ps_filter(Some("blockers")),
        Some(WorkerFilter::Blockers)
    );
    assert_eq!(
        parse_ps_filter(Some("dependencies")),
        Some(WorkerFilter::Dependencies)
    );
    assert_eq!(
        parse_ps_filter(Some("deps")),
        Some(WorkerFilter::Dependencies)
    );
    assert_eq!(
        parse_ps_filter(Some("blocking")),
        Some(WorkerFilter::Blockers)
    );
    assert_eq!(
        parse_ps_filter(Some("prereqs")),
        Some(WorkerFilter::Blockers)
    );
    assert_eq!(parse_ps_filter(Some("agents")), Some(WorkerFilter::Agents));
    assert_eq!(parse_ps_filter(Some("shells")), Some(WorkerFilter::Shells));
    assert_eq!(
        parse_ps_filter(Some("services")),
        Some(WorkerFilter::Services)
    );
    assert_eq!(
        parse_ps_filter(Some("capabilities")),
        Some(WorkerFilter::Capabilities)
    );
    assert_eq!(
        parse_ps_filter(Some("caps")),
        Some(WorkerFilter::Capabilities)
    );
    assert_eq!(
        parse_ps_filter(Some("terminals")),
        Some(WorkerFilter::Terminals)
    );
    assert_eq!(
        parse_ps_filter(Some("actions")),
        Some(WorkerFilter::Actions)
    );
    assert_eq!(
        parse_ps_filter(Some("suggestions")),
        Some(WorkerFilter::Actions)
    );
    assert_eq!(parse_ps_filter(Some("clean")), None);
    assert_eq!(parse_ps_filter(Some("unknown")), None);
}

#[test]
fn ps_focus_capability_parser_accepts_capability_selector() {
    assert_eq!(
        parse_ps_focus_capability(&["@api.http"], ":ps actions").expect("parse capability"),
        "api.http"
    );
    assert!(parse_ps_focus_capability(&["api.http"], ":ps actions").is_err());
    assert!(parse_ps_focus_capability(&["@api.http", "@db.redis"], ":ps actions").is_err());
}

#[test]
fn ps_dependency_filter_parser_accepts_dependency_issue_aliases() {
    use crate::orchestration_view::DependencyFilter;

    assert_eq!(
        parse_ps_dependency_filter(None),
        Some(DependencyFilter::All)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("blocking")),
        Some(DependencyFilter::Blocking)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("blockers")),
        Some(DependencyFilter::Blocking)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("sidecars")),
        Some(DependencyFilter::Sidecars)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("missing")),
        Some(DependencyFilter::Missing)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("booting")),
        Some(DependencyFilter::Booting)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("ambiguous")),
        Some(DependencyFilter::Ambiguous)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("conflicts")),
        Some(DependencyFilter::Ambiguous)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("satisfied")),
        Some(DependencyFilter::Satisfied)
    );
    assert_eq!(
        parse_ps_dependency_filter(Some("ready")),
        Some(DependencyFilter::Satisfied)
    );
    assert_eq!(parse_ps_dependency_filter(Some("weird")), None);
}

#[test]
fn ps_dependency_selector_accepts_optional_capability_reference() {
    use crate::orchestration_view::DependencyFilter;
    use crate::orchestration_view::DependencySelection;

    assert_eq!(
        parse_ps_dependency_selector(&["missing", "@api.http"]).expect("selector"),
        DependencySelection {
            filter: DependencyFilter::Missing,
            capability: Some("api.http".to_string()),
        }
    );
    assert_eq!(
        parse_ps_dependency_selector(&["@api.http"]).expect("selector"),
        DependencySelection {
            filter: DependencyFilter::All,
            capability: Some("api.http".to_string()),
        }
    );
    assert!(parse_ps_dependency_selector(&["missing", "weird"]).is_err());
    assert!(parse_ps_dependency_selector(&["missing", "@api.http", "@frontend.dev"]).is_err());
}

#[test]
fn ps_capability_filter_parser_accepts_issue_aliases() {
    use crate::background_shells::BackgroundShellCapabilityIssueClass;

    assert_eq!(parse_ps_capability_issue_filter(None), Some(None));
    assert_eq!(parse_ps_capability_issue_filter(Some("all")), Some(None));
    assert_eq!(
        parse_ps_capability_issue_filter(Some("healthy")),
        Some(Some(BackgroundShellCapabilityIssueClass::Healthy))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("ok")),
        Some(Some(BackgroundShellCapabilityIssueClass::Healthy))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("missing")),
        Some(Some(BackgroundShellCapabilityIssueClass::Missing))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("booting")),
        Some(Some(BackgroundShellCapabilityIssueClass::Booting))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("ambiguous")),
        Some(Some(BackgroundShellCapabilityIssueClass::Ambiguous))
    );
    assert_eq!(
        parse_ps_capability_issue_filter(Some("conflicts")),
        Some(Some(BackgroundShellCapabilityIssueClass::Ambiguous))
    );
    assert_eq!(parse_ps_capability_issue_filter(Some("weird")), None);
}

#[test]
fn ps_service_selector_accepts_optional_capability_reference() {
    use crate::background_shells::BackgroundShellServiceIssueClass;

    assert_eq!(
        parse_ps_service_selector(&["ready", "@api.http"]).expect("selector"),
        (
            Some(BackgroundShellServiceIssueClass::Ready),
            Some("api.http".to_string()),
        )
    );
    assert_eq!(
        parse_ps_service_selector(&["@api.http"]).expect("selector"),
        (None, Some("api.http".to_string()))
    );
    assert!(parse_ps_service_selector(&["ready", "weird"]).is_err());
    assert!(parse_ps_service_selector(&["ready", "@bad!"]).is_err());
    assert!(parse_ps_service_selector(&["ready", "@api.http", "@frontend.dev"]).is_err());
}

#[test]
fn ps_service_filter_parser_accepts_issue_aliases() {
    use crate::background_shells::BackgroundShellServiceIssueClass;

    assert_eq!(parse_ps_service_issue_filter(None), Some(None));
    assert_eq!(parse_ps_service_issue_filter(Some("all")), Some(None));
    assert_eq!(
        parse_ps_service_issue_filter(Some("ready")),
        Some(Some(BackgroundShellServiceIssueClass::Ready))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("healthy")),
        Some(Some(BackgroundShellServiceIssueClass::Ready))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("booting")),
        Some(Some(BackgroundShellServiceIssueClass::Booting))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("untracked")),
        Some(Some(BackgroundShellServiceIssueClass::Untracked))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("conflicts")),
        Some(Some(BackgroundShellServiceIssueClass::Conflicts))
    );
    assert_eq!(
        parse_ps_service_issue_filter(Some("ambiguous")),
        Some(Some(BackgroundShellServiceIssueClass::Conflicts))
    );
    assert_eq!(parse_ps_service_issue_filter(Some("weird")), None);
}

#[test]
fn clean_target_parser_accepts_scoped_cleanup_aliases() {
    use crate::dispatch_command_session_ps::CleanSelection;
    use crate::dispatch_command_session_ps::CleanTarget;

    assert_eq!(parse_clean_target(None), Some(CleanTarget::All));
    assert_eq!(parse_clean_target(Some("all")), Some(CleanTarget::All));
    assert_eq!(
        parse_clean_target(Some("blockers")),
        Some(CleanTarget::Blockers)
    );
    assert_eq!(
        parse_clean_target(Some("blocking")),
        Some(CleanTarget::Blockers)
    );
    assert_eq!(
        parse_clean_target(Some("prereqs")),
        Some(CleanTarget::Blockers)
    );
    assert_eq!(
        parse_clean_target(Some("shells")),
        Some(CleanTarget::Shells)
    );
    assert_eq!(
        parse_clean_target(Some("services")),
        Some(CleanTarget::Services)
    );
    assert_eq!(
        parse_clean_target(Some("terminals")),
        Some(CleanTarget::Terminals)
    );
    assert_eq!(parse_clean_target(Some("agents")), None);
    assert_eq!(parse_clean_target(Some("unknown")), None);

    assert_eq!(
        parse_clean_selection(&["blockers", "@api.http"], ":clean")
            .expect("parse blocker clean selector"),
        CleanSelection {
            target: CleanTarget::Blockers,
            capability: Some("api.http".to_string())
        }
    );
    assert_eq!(
        parse_clean_selection(&["services", "@api.http"], ":clean").expect("parse clean selector"),
        CleanSelection {
            target: CleanTarget::Services,
            capability: Some("api.http".to_string())
        }
    );
    assert!(parse_clean_selection(&["services", "api.http"], ":clean").is_err());
    assert!(parse_clean_selection(&["shells", "@api.http"], ":clean").is_err());
}

#[test]
fn ps_command_can_poll_and_terminate_specific_background_shell_jobs() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "printf 'alpha\\nbeta\\n'", "intent": "service"}),
            "/tmp",
        )
        .expect("start pollable shell");
    std::thread::sleep(Duration::from_millis(50));

    handle_ps_command(
        "poll 1",
        &["poll", "1"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("poll background shell");

    assert_eq!(state.background_shells.job_count(), 1);
    let polled = state
        .background_shells
        .poll_job("bg-1", 0, 200)
        .expect("poll shell directly");
    assert!(polled.contains("Job: bg-1"));
    assert!(polled.contains("alpha"));

    handle_ps_command(
        "terminate bg-1",
        &["terminate", "bg-1"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("terminate background shell");

    let after = state
        .background_shells
        .poll_job("bg-1", 0, 20)
        .expect("poll after terminate");
    assert!(after.contains("Status: terminated") || after.contains("Status: completed"));
}

#[test]
fn ps_command_can_alias_and_reuse_background_shell_job_references() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "printf 'alpha\\n'", "intent": "service", "label": "dev server"}),
            "/tmp",
        )
        .expect("start aliasable shell");
    std::thread::sleep(Duration::from_millis(50));

    handle_ps_command(
        "alias 1 dev.api",
        &["alias", "1", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("alias background shell");
    assert_eq!(
        state
            .background_shells
            .resolve_job_reference("dev.api")
            .expect("resolve alias"),
        "bg-1"
    );

    handle_ps_command(
        "poll dev.api",
        &["poll", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("poll aliased shell");
    let polled = state
        .background_shells
        .poll_job("bg-1", 0, 200)
        .expect("poll shell directly");
    assert!(polled.contains("Alias: dev.api"));

    handle_ps_command(
        "unalias dev.api",
        &["unalias", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("clear alias");
    assert!(
        state
            .background_shells
            .resolve_job_reference("dev.api")
            .is_err()
    );
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_alias_and_unalias_background_shell_job_by_capability_reference() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "printf 'ready\\n'; sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "label": "api"
            }),
            "/tmp",
        )
        .expect("start service shell");
    std::thread::sleep(Duration::from_millis(50));

    handle_ps_command(
        "alias @api.http dev.api",
        &["alias", "@api.http", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("alias background shell by capability");
    assert_eq!(
        state
            .background_shells
            .resolve_job_reference("dev.api")
            .expect("resolve alias"),
        "bg-1"
    );

    handle_ps_command(
        "unalias @api.http",
        &["unalias", "@api.http"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("clear alias by capability");
    assert!(
        state
            .background_shells
            .resolve_job_reference("dev.api")
            .is_err()
    );
    let polled = state
        .background_shells
        .poll_job("bg-1", 0, 200)
        .expect("poll shell directly");
    assert!(!polled.contains("Alias: dev.api"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_clean_services_can_target_one_capability() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "api a",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first api provider");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "api b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second api provider");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "service",
                "label": "db",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db provider");

    handle_ps_command(
        "clean services @api.http",
        &["clean", "services", "@api.http"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("clean services by capability");

    let rendered = state
        .background_shells
        .render_service_shells_for_ps_filtered(None, None)
        .expect("render remaining services")
        .join("\n");
    assert!(!rendered.contains("api a"));
    assert!(!rendered.contains("api b"));
    assert!(rendered.contains("db"));
}

#[test]
fn ps_clean_blockers_can_target_one_capability() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "prerequisite",
                "label": "api wait",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api blocker");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.5",
                "intent": "prerequisite",
                "label": "db wait",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    handle_ps_command(
        "clean blockers @api.http",
        &["clean", "blockers", "@api.http"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("clean blockers by capability");

    let rendered = state
        .background_shells
        .capability_dependency_summaries()
        .into_iter()
        .map(|summary| format!("{} -> {}", summary.job_id, summary.capability))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!rendered.contains("api.http"));
    assert!(rendered.contains("db.redis"));
}

#[test]
fn ps_command_can_send_input_to_aliased_background_shell_job() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({"command": if cfg!(windows) { "more" } else { "cat" }, "intent": "service"}),
            "/tmp",
        )
        .expect("start interactive shell");

    handle_ps_command(
        "alias 1 dev.api",
        &["alias", "1", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("alias background shell");

    handle_ps_command(
        "send dev.api hello there from ps",
        &["send", "dev.api", "hello", "there", "from", "ps"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("send stdin");

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = state
            .background_shells
            .poll_job("bg-1", 0, 200)
            .expect("poll shell directly");
        if rendered.contains("hello there from ps") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("hello there from ps"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn service_capability_reference_can_drive_ps_attach() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000"
            }),
            "/tmp",
        )
        .expect("start service shell");

    assert_eq!(
        state
            .background_shells
            .resolve_job_reference("@api.http")
            .expect("resolve service capability"),
        "bg-1"
    );

    handle_ps_command(
        "attach @api.http",
        &["attach", "@api.http"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("attach service by capability");

    let attached = state
        .background_shells
        .attach_for_operator("bg-1")
        .expect("attach directly");
    assert!(attached.contains("Capabilities: api.http"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_service_capability_index() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http", "frontend.dev"],
            }),
            "/tmp",
        )
        .expect("start service shell");

    handle_ps_command(
        "capabilities",
        &["capabilities"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render capability index");

    let rendered = state
        .background_shells
        .render_service_capabilities_for_ps()
        .expect("capability index");
    let joined = rendered.join("\n");
    assert!(joined.contains("Service capability index:"));
    assert!(joined.contains("@api.http -> bg-1"));
    assert!(joined.contains("@frontend.dev -> bg-1"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_single_service_capability_detail() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
            }),
            "/tmp",
        )
        .expect("start service shell");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"],
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    handle_ps_command(
        "capabilities @api.http",
        &["capabilities", "@api.http"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render capability detail");

    let rendered = state
        .background_shells
        .render_single_service_capability_for_ps("@api.http")
        .expect("capability detail")
        .join("\n");
    assert!(rendered.contains("Service capability: @api.http"));
    assert!(rendered.contains("Providers:"));
    assert!(rendered.contains("bg-1  [untracked]"));
    assert!(rendered.contains("bg-2  [satisfied]  blocking=yes"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_capability_index_by_issue_class() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"],
            }),
            "/tmp",
        )
        .expect("start dependent shell");

    handle_ps_command(
        "capabilities missing",
        &["capabilities", "missing"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render filtered capability index");

    let rendered = state
        .background_shells
        .render_service_capabilities_for_ps_filtered(Some(
            crate::background_shells::BackgroundShellCapabilityIssueClass::Missing,
        ))
        .expect("capability filter")
        .join("\n");
    assert!(rendered.contains("@api.http -> <missing provider> [missing]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_capability_index_by_untracked_issue_class() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
            }),
            "/tmp",
        )
        .expect("start service shell");

    handle_ps_command(
        "capabilities untracked",
        &["capabilities", "untracked"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render filtered untracked capability index");

    let rendered = state
        .background_shells
        .render_service_capabilities_for_ps_filtered(Some(
            crate::background_shells::BackgroundShellCapabilityIssueClass::Untracked,
        ))
        .expect("capability filter")
        .join("\n");
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_dependencies_by_capability() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"],
            }),
            "/tmp",
        )
        .expect("start api dependent shell");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "observation",
                "dependsOnCapabilities": ["db.redis"],
            }),
            "/tmp",
        )
        .expect("start redis dependent shell");

    handle_ps_command(
        "dependencies missing @api.http",
        &["dependencies", "missing", "@api.http"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render filtered dependency view");

    let rendered = crate::orchestration_view::render_orchestration_dependencies(
        &state,
        &crate::orchestration_view::DependencySelection {
            filter: crate::orchestration_view::DependencyFilter::Missing,
            capability: Some("api.http".to_string()),
        },
    );
    assert!(rendered.contains("Dependencies (@api.http):"));
    assert!(rendered.contains("shell:bg-1 -> capability:@api.http"));
    assert!(!rendered.contains("db.redis"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_service_shells_by_state() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) {
                    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "booting svc",
                "capabilities": ["svc.booting"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service shell");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) {
                    "echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "ready svc",
                "capabilities": ["svc.ready"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start ready service shell");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "untracked svc",
                "capabilities": ["svc.untracked"]
            }),
            "/tmp",
        )
        .expect("start untracked service shell");

    state
        .background_shells
        .wait_ready_for_operator("bg-2", 2_000)
        .expect("wait for ready service");

    handle_ps_command(
        "services ready",
        &["services", "ready"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render ready service shells");

    let ready = state
        .background_shells
        .render_service_shells_for_ps_filtered(
            Some(crate::background_shells::BackgroundShellServiceIssueClass::Ready),
            None,
        )
        .expect("ready services")
        .join("\n");
    assert!(ready.contains("ready svc"));
    assert!(!ready.contains("booting svc"));

    handle_ps_command(
        "services booting",
        &["services", "booting"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render booting service shells");

    let booting = state
        .background_shells
        .render_service_shells_for_ps_filtered(
            Some(crate::background_shells::BackgroundShellServiceIssueClass::Booting),
            None,
        )
        .expect("booting services")
        .join("\n");
    assert!(booting.contains("booting svc"));
    assert!(!booting.contains("ready svc"));

    handle_ps_command(
        "services untracked",
        &["services", "untracked"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render untracked service shells");

    let untracked = state
        .background_shells
        .render_service_shells_for_ps_filtered(
            Some(crate::background_shells::BackgroundShellServiceIssueClass::Untracked),
            None,
        )
        .expect("untracked services")
        .join("\n");
    assert!(untracked.contains("untracked svc"));
    assert!(!untracked.contains("booting svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_service_shells_by_capability() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "frontend svc",
                "capabilities": ["frontend.dev"]
            }),
            "/tmp",
        )
        .expect("start frontend service");

    handle_ps_command(
        "services @api.http",
        &["services", "@api.http"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render focused service shells");

    let rendered = state
        .background_shells
        .render_service_shells_for_ps_filtered(None, Some("@api.http"))
        .expect("service filter")
        .join("\n");
    assert!(rendered.contains("api svc"));
    assert!(rendered.contains("api.http"));
    assert!(!rendered.contains("frontend svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_conflicting_service_shells() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "conflict a",
                "capabilities": ["svc.conflict"]
            }),
            "/tmp",
        )
        .expect("start first conflicting service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "conflict b",
                "capabilities": ["svc.conflict"]
            }),
            "/tmp",
        )
        .expect("start second conflicting service");

    handle_ps_command(
        "services conflicts",
        &["services", "conflicts"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render conflicting service shells");

    let rendered = state
        .background_shells
        .render_service_shells_for_ps_filtered(
            Some(crate::background_shells::BackgroundShellServiceIssueClass::Conflicts),
            None,
        )
        .expect("conflict services")
        .join("\n");
    assert!(rendered.contains("conflict a"));
    assert!(rendered.contains("conflict b"));
    assert!(rendered.contains("Capability conflicts:"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_service_attachment_metadata_for_aliased_job() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "attachHint": "Send HTTP requests to /health",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
                        "example": "curl http://127.0.0.1:4000/health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    handle_ps_command(
        "alias 1 dev.api",
        &["alias", "1", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("alias background shell");

    handle_ps_command(
        "attach dev.api",
        &["attach", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("render service attachment");

    let rendered = state
        .background_shells
        .attach_for_operator("bg-1")
        .expect("attachment summary");
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:4000"));
    assert!(rendered.contains("Attach hint: Send HTTP requests to /health"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_wait_for_service_readiness_by_alias() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) {
                    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start delayed-ready service shell");

    handle_ps_command(
        "alias 1 dev.api",
        &["alias", "1", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("alias background shell");

    handle_ps_command(
        "wait dev.api 2000",
        &["wait", "dev.api", "2000"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("wait for service readiness");

    let rendered = state
        .background_shells
        .wait_ready_for_operator("bg-1", 2_000)
        .expect("re-wait after ready");
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("ready"));
    let _ = state.background_shells.terminate_all_running();
}
