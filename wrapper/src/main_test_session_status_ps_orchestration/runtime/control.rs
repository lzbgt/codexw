use super::*;

#[path = "control/jobs.rs"]
mod jobs;
#[path = "control/services.rs"]
mod services;

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
