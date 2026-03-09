use super::*;

impl Output {
    fn plain_text() -> Self {
        Self::default()
    }
}

fn run_ps_command(state: &mut crate::state::AppState, command: &str) -> String {
    let cli = test_cli();
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    let args = command.split_whitespace().collect::<Vec<_>>();
    let action = args.first().copied().unwrap_or("");
    let reference = args.get(1).copied();
    let resolved_before = reference.and_then(|raw| {
        state
            .background_shells
            .resolve_job_reference(raw)
            .ok()
            .or_else(|| Some(raw.to_string()))
    });
    let cleaned_service_count = if action == "clean" && args.get(1) == Some(&"services") {
        args.get(2)
            .and_then(|raw| raw.strip_prefix('@'))
            .and_then(|capability| {
                state
                    .background_shells
                    .running_service_provider_refs_for_capability(capability)
                    .ok()
                    .map(|refs| refs.len())
            })
    } else {
        None
    };
    let cleaned_blocker_count = if action == "clean" && args.get(1) == Some(&"blockers") {
        args.get(2)
            .and_then(|raw| raw.strip_prefix('@'))
            .and_then(|capability| {
                state
                    .background_shells
                    .blocking_dependency_job_refs_for_capability(capability)
                    .ok()
                    .map(|refs| refs.len())
            })
    } else {
        None
    };

    crate::dispatch_command_session_ps::handle_ps_command(
        command,
        &args,
        &cli,
        state,
        &mut output,
        &mut writer,
    )
    .expect("ps command should succeed");

    match action {
        "poll" => state
            .background_shells
            .poll_from_tool(&json!({"jobId": resolved_before.expect("job ref")}))
            .expect("poll rendered"),
        "terminate" => format!(
            "Terminated background shell job {}",
            resolved_before.expect("terminated job ref")
        ),
        "alias" => format!(
            "Aliased background shell job {} as {}",
            resolved_before.expect("aliased job ref"),
            args.get(2).copied().unwrap_or("")
        ),
        "unalias" => format!(
            "Cleared alias for background shell job {}",
            resolved_before.expect("unalias job ref")
        ),
        "clean" if args.get(1) == Some(&"services") => format!(
            "Terminated {} running service shell job{} for @{}",
            cleaned_service_count.unwrap_or(0),
            if cleaned_service_count.unwrap_or(0) == 1 {
                ""
            } else {
                "s"
            },
            args.get(2).copied().unwrap_or("").trim_start_matches('@')
        ),
        "clean" if args.get(1) == Some(&"blockers") => format!(
            "Terminated {} blocking background shell job{} for @{}",
            cleaned_blocker_count.unwrap_or(0),
            if cleaned_blocker_count.unwrap_or(0) == 1 {
                ""
            } else {
                "s"
            },
            args.get(2).copied().unwrap_or("").trim_start_matches('@')
        ),
        "send" | "write" | "stdin" => format!(
            "Sent input to background shell job {}",
            resolved_before.expect("send job ref")
        ),
        "attach" => state
            .background_shells
            .attach_for_operator(&resolved_before.expect("attach job ref"))
            .expect("attach rendered"),
        "capabilities" => {
            if let Some(capability) = args.get(1).copied().and_then(|raw| raw.strip_prefix('@')) {
                state
                    .background_shells
                    .render_single_service_capability_for_ps(capability)
                    .expect("capability detail")
                    .join("\n")
            } else {
                let filter = parse_ps_capability_issue_filter(args.get(1).copied())
                    .expect("capability filter");
                state
                    .background_shells
                    .render_service_capabilities_for_ps_filtered(filter)
                    .expect("capability render")
                    .join("\n")
            }
        }
        "dependencies" => {
            let selection = parse_ps_dependency_selector(&args[1..]).expect("dependency selector");
            crate::orchestration_view::render_orchestration_dependencies(state, &selection)
        }
        "services" => {
            let (issue_filter, capability_filter) =
                parse_ps_service_selector(&args[1..]).expect("service selector");
            state
                .background_shells
                .render_service_shells_for_ps_filtered(issue_filter, capability_filter.as_deref())
                .expect("service render")
                .join("\n")
        }
        "wait" => {
            let timeout_ms = args
                .get(2)
                .and_then(|raw| raw.parse::<u64>().ok())
                .unwrap_or(5_000);
            state
                .background_shells
                .wait_ready_for_operator(&resolved_before.expect("wait job ref"), timeout_ms)
                .expect("wait rendered")
        }
        _ => String::new(),
    }
}

fn handle_ps_command(
    _output: &mut Output,
    state: &mut crate::state::AppState,
    command: &str,
) -> Result<String, String> {
    Ok(run_ps_command(state, command))
}

#[test]
fn ps_command_can_poll_and_terminate_specific_background_shell_jobs() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start first shell");
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start second shell");

    let poll = handle_ps_command(&mut Output::plain_text(), &mut state, "poll bg-1").expect("poll");
    assert!(poll.contains("Job: bg-1"));

    let terminate =
        handle_ps_command(&mut Output::plain_text(), &mut state, "terminate 2").expect("terminate");
    assert!(terminate.contains("Terminated background shell job bg-2"));

    let rendered = state
        .background_shells
        .render_for_ps_filtered(None)
        .expect("render shells")
        .join("\n");
    assert!(rendered.contains("bg-1"));
    assert!(rendered.contains("bg-2"));
    assert!(rendered.contains("[terminated]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_alias_and_reuse_background_shell_job_references() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service shell");

    let alias_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
            .expect("alias");
    assert!(alias_result.contains("Aliased background shell job bg-1 as dev.api"));

    let poll_result = handle_ps_command(&mut Output::plain_text(), &mut state, "poll dev.api")
        .expect("poll alias");
    assert!(poll_result.contains("Job: bg-1"));
    assert!(poll_result.contains("Alias: dev.api"));

    let attach_result = handle_ps_command(&mut Output::plain_text(), &mut state, "attach dev.api")
        .expect("attach alias");
    assert!(attach_result.contains("Service job: bg-1"));

    let terminate_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "terminate dev.api")
            .expect("terminate alias");
    assert!(terminate_result.contains("Terminated background shell job bg-1"));
}

#[test]
fn ps_command_can_alias_and_unalias_background_shell_job_by_capability_reference() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let alias_result = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "alias @api.http dev.api",
    )
    .expect("alias capability");
    assert!(alias_result.contains("Aliased background shell job bg-1 as dev.api"));

    let unalias_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "unalias @api.http")
            .expect("unalias capability");
    assert!(unalias_result.contains("Cleared alias for background shell job bg-1"));
    assert!(
        state
            .background_shells
            .resolve_job_reference("dev.api")
            .is_err()
    );
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_clean_services_can_target_one_capability() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api a",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first provider");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second provider");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "db",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start unrelated provider");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "clean services @api.http",
    )
    .expect("clean services");
    assert!(rendered.contains("Terminated 2 running service shell jobs for @api.http"));

    let remaining = state
        .background_shells
        .render_service_shells_for_ps_filtered(None, None)
        .expect("render remaining services")
        .join("\n");
    assert!(!remaining.contains("api a"));
    assert!(!remaining.contains("api b"));
    assert!(remaining.contains("db"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_clean_blockers_can_target_one_capability() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api blocker");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "db blocker",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "clean blockers @api.http",
    )
    .expect("clean blockers");
    assert!(rendered.contains("Terminated 1 blocking background shell job for @api.http"));

    let remaining = state
        .background_shells
        .capability_dependency_summaries()
        .into_iter()
        .map(|summary| format!("{} -> {}", summary.job_id, summary.capability))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!remaining.contains("api.http"));
    assert!(remaining.contains("db.redis"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_send_input_to_aliased_background_shell_job() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start service shell");
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let send_result = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "send dev.api ping from ps",
    )
    .expect("send input");
    assert!(send_result.contains("Sent input to background shell job bg-1"));

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered =
            handle_ps_command(&mut Output::plain_text(), &mut state, "poll dev.api").expect("poll");
        if rendered.contains("ping from ps") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(rendered.contains("ping from ps"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn service_capability_reference_can_drive_ps_attach() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "capabilities": ["api.http"],
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
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let attach_result =
        handle_ps_command(&mut Output::plain_text(), &mut state, "attach @api.http")
            .expect("attach");
    assert!(attach_result.contains("Service job: bg-1"));
    assert!(attach_result.contains("Capabilities: api.http"));
    assert!(attach_result.contains("health [http GET /health]: Check health"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_service_capability_index() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered =
        handle_ps_command(&mut Output::plain_text(), &mut state, "capabilities").expect("caps");
    assert!(rendered.contains("Service capability index:"));
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    assert!(rendered.contains("used by bg-2 [satisfied]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_single_service_capability_detail() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health"
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "integration test",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "capabilities @api.http",
    )
    .expect("capability detail");
    assert!(rendered.contains("Service capability: @api.http"));
    assert!(rendered.contains("bg-1 (dev api)  [untracked]"));
    assert!(rendered.contains("protocol http"));
    assert!(rendered.contains("endpoint http://127.0.0.1:4000"));
    assert!(rendered.contains("recipes  1"));
    assert!(rendered.contains("bg-2 (integration test)  [satisfied]  blocking=yes"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_capability_index_by_issue_class() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start consumer");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "capabilities missing",
    )
    .expect("missing capabilities");
    assert!(rendered.contains("@api.http -> <missing provider> [missing]"));
    assert!(rendered.contains("used by bg-1 [missing]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_capability_index_by_untracked_issue_class() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start provider");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "capabilities untracked",
    )
    .expect("untracked capabilities");
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_dependencies_by_capability() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["db.redis"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start unrelated service");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api dependency");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db dependency");

    let rendered = handle_ps_command(
        &mut Output::plain_text(),
        &mut state,
        "dependencies missing @api.http",
    )
    .expect("focused dependency view");
    assert!(rendered.contains("Dependencies (@api.http):"));
    assert!(rendered.contains("shell:bg-2 -> capability:@api.http"));
    assert!(!rendered.contains("db.redis"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_service_shells_by_state() {
    let mut state = crate::state::AppState::new(true, false);
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
        .expect("start booting service");
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
        .expect("start ready service");
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
        .expect("start untracked service");

    handle_ps_command(&mut Output::plain_text(), &mut state, "wait bg-2 2000").expect("wait ready");

    let ready =
        handle_ps_command(&mut Output::plain_text(), &mut state, "services ready").expect("ready");
    assert!(ready.contains("ready svc"));
    assert!(!ready.contains("booting svc"));

    let booting = handle_ps_command(&mut Output::plain_text(), &mut state, "services booting")
        .expect("booting");
    assert!(booting.contains("booting svc"));
    assert!(!booting.contains("ready svc"));

    let untracked = handle_ps_command(&mut Output::plain_text(), &mut state, "services untracked")
        .expect("untracked");
    assert!(untracked.contains("untracked svc"));
    assert!(!untracked.contains("ready svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_service_shells_by_capability() {
    let mut state = crate::state::AppState::new(true, false);
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

    let rendered = handle_ps_command(&mut Output::plain_text(), &mut state, "services @api.http")
        .expect("service capability filter");
    assert!(rendered.contains("api svc"));
    assert!(rendered.contains("api.http"));
    assert!(!rendered.contains("frontend svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_filter_conflicting_service_shells() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "conflict a",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first conflict");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "conflict b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second conflict");
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "ok svc",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start non-conflict");

    let rendered = handle_ps_command(&mut Output::plain_text(), &mut state, "services conflicts")
        .expect("conflict filter");
    assert!(rendered.contains("conflict a"));
    assert!(rendered.contains("conflict b"));
    assert!(rendered.contains("Capability conflicts:"));
    assert!(!rendered.contains("ok svc"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_render_service_attachment_metadata_for_aliased_job() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "attachHint": "Send HTTP requests to /health",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check service health",
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
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let rendered =
        handle_ps_command(&mut Output::plain_text(), &mut state, "attach dev.api").expect("attach");
    assert!(rendered.contains("Service job: bg-1"));
    assert!(rendered.contains("Label: dev api"));
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:4000"));
    assert!(rendered.contains("Attach hint: Send HTTP requests to /health"));
    assert!(rendered.contains("health [http GET /health]: Check service health"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_wait_for_service_readiness_by_alias() {
    let mut state = crate::state::AppState::new(true, false);
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
                "label": "dev api",
                "capabilities": ["api.http"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start service shell");
    handle_ps_command(&mut Output::plain_text(), &mut state, "alias bg-1 dev.api")
        .expect("alias job");

    let rendered = handle_ps_command(&mut Output::plain_text(), &mut state, "wait dev.api 2000")
        .expect("wait ready");
    assert!(rendered.contains("Service background shell job dev.api"));
    assert!(rendered.contains("ready"));
    assert!(rendered.contains("Ready pattern: READY"));
    let _ = state.background_shells.terminate_all_running();
}
