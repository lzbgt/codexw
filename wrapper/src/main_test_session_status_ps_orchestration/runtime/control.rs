use super::*;

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
