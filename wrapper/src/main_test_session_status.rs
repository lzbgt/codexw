use crate::Cli;
use crate::background_terminals::background_terminal_count;
use crate::background_terminals::render_background_terminals;
use crate::dispatch_command_session_ps::handle_ps_command;
use crate::dispatch_command_session_ps::parse_clean_target;
use crate::dispatch_command_session_ps::parse_ps_filter;
use crate::events::handle_realtime_notification;
use crate::notification_item_buffers::handle_buffer_update;
use crate::notification_item_completion::render_item_completed;
use crate::notification_item_status::handle_status_update;
use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_view::CachedAgentThreadSummary;
use crate::orchestration_view::WorkerFilter;
use crate::output::Output;
use crate::prompt_state::render_prompt_status;
use crate::session_prompt_status_active::spinner_frame;
use crate::session_snapshot_overview::render_status_overview;
use crate::session_snapshot_runtime::render_status_runtime;
use crate::transcript_status_summary::summarize_thread_status_for_display;
use serde_json::json;
use std::process::ChildStdin;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;

#[test]
fn thread_status_summary_prefers_human_flags() {
    assert_eq!(
        summarize_thread_status_for_display(&json!({
            "status": {"type": "active", "activeFlags": ["waitingOnApproval"]}
        })),
        Some("waiting on approval".to_string())
    );
    assert_eq!(
        summarize_thread_status_for_display(&json!({
            "status": {"type": "idle", "activeFlags": []}
        })),
        Some("ready".to_string())
    );
}

#[test]
fn prompt_status_uses_active_detail_when_present() {
    let mut state = crate::state::AppState::new(true, false);
    state.turn_running = true;
    state.started_turn_count = 2;
    state.last_status_line = Some("waiting on approval".to_string());
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("waiting on approval"));
}

#[test]
fn active_spinner_uses_codex_braille_frames() {
    assert_eq!(spinner_frame(None), "⠋");
    let now = Instant::now();
    assert_eq!(spinner_frame(Some(now - Duration::from_millis(100))), "⠙");
    assert_eq!(spinner_frame(Some(now - Duration::from_millis(700))), "⠇");
}

#[test]
fn prompt_status_mentions_realtime_when_active() {
    let mut state = crate::state::AppState::new(true, false);
    state.realtime_active = true;
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("realtime"));
}

#[test]
fn prompt_status_mentions_startup_resume_picker() {
    let mut state = crate::state::AppState::new(true, false);
    state.startup_resume_picker = true;
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("resume picker"));
    assert!(rendered.contains(" | "));
    assert!(rendered.contains("/new"));
}

#[test]
fn prompt_status_ready_includes_collaboration_and_personality() {
    let mut state = crate::state::AppState::new(true, false);
    state.completed_turn_count = 3;
    state.active_personality = Some("pragmatic".to_string());
    state.active_collaboration_mode = Some(crate::collaboration_preset::CollaborationModePreset {
        name: "Plan".to_string(),
        mode_kind: Some("plan".to_string()),
        model: Some("gpt-5-codex".to_string()),
        reasoning_effort: Some(Some("high".to_string())),
    });
    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("plan mode"));
    assert!(rendered.contains("Pragmatic"));
    assert!(rendered.contains("3 turns"));
    assert!(rendered.contains(" | "));
}

#[test]
fn status_snapshot_includes_realtime_fields() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.realtime_active = true;
    state.realtime_session_id = Some("rt-1".to_string());
    state.realtime_prompt = Some("hello world".to_string());
    state.realtime_last_error = Some("bad gateway".to_string());
    let cli = crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        prompt: Vec::new(),
    });
    let mut lines = render_status_overview(&cli, "/tmp/project", &state);
    lines.extend(render_status_runtime(&cli, &state));
    let rendered = lines.join("\n");
    assert!(rendered.contains("realtime        true"));
    assert!(rendered.contains("realtime id     rt-1"));
    assert!(rendered.contains("realtime prompt hello world"));
    assert!(rendered.contains("realtime error  bad gateway"));
}

#[test]
fn resetting_thread_context_clears_stream_buffers() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .command_output_buffers
        .insert("cmd-1".to_string(), "out".to_string());
    state
        .file_output_buffers
        .insert("file-1".to_string(), "diff".to_string());
    state.process_output_buffers.insert(
        "proc-1".to_string(),
        crate::state::ProcessOutputBuffer {
            stdout: "stdout".to_string(),
            stderr: "stderr".to_string(),
        },
    );
    state.last_agent_message = Some("reply".to_string());
    state.last_turn_diff = Some("diff".to_string());
    state.last_status_line = Some("running".to_string());
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["thread-agent-1".to_string()],
            prompt: Some("inspect auth".to_string()),
            agent_statuses: std::collections::BTreeMap::from([(
                "thread-agent-1".to_string(),
                "running".to_string(),
            )]),
        },
    );

    state.reset_thread_context();

    assert!(state.command_output_buffers.is_empty());
    assert!(state.file_output_buffers.is_empty());
    assert!(state.process_output_buffers.is_empty());
    assert!(state.last_agent_message.is_none());
    assert!(state.last_turn_diff.is_none());
    assert!(state.last_status_line.is_none());
    assert!(state.live_agent_tasks.is_empty());
    assert!(!state.startup_resume_picker);
}

fn test_cli() -> Cli {
    crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        prompt: Vec::new(),
    })
}

fn spawn_sink_stdin() -> ChildStdin {
    #[cfg(unix)]
    let mut child = Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn sink");
    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args(["/C", "more >NUL"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn sink");
    child.stdin.take().expect("child stdin")
}

#[test]
fn completed_command_execution_clears_matching_running_status() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line =
        Some("running find frontend/src/components -maxdepth 2 -type f | sort".to_string());
    state.command_output_buffers.insert(
        "cmd-1".to_string(),
        "frontend/src/components/A.tsx\n".to_string(),
    );
    let mut output = Output::default();

    render_item_completed(
        &cli,
        &serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "cmd-1",
                "command": "find frontend/src/components -maxdepth 2 -type f | sort",
                "status": "completed",
                "exitCode": 0
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("render completed command");

    assert!(state.last_status_line.is_none());
}

#[test]
fn completed_command_execution_keeps_newer_status_line() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line = Some("waiting on approval".to_string());
    let mut output = Output::default();

    render_item_completed(
        &cli,
        &serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "cmd-1",
                "command": "find frontend/src/components -maxdepth 2 -type f | sort",
                "status": "completed",
                "exitCode": 0,
                "aggregatedOutput": ""
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("render completed command");

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on approval")
    );
}

#[test]
fn active_thread_status_without_flags_clears_stale_detail() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line = Some("waiting on approval".to_string());
    let mut output = Output::default();

    handle_realtime_notification(
        &crate::rpc::RpcNotification {
            method: "thread/status/changed".to_string(),
            params: serde_json::json!({
                "status": {"type": "active", "activeFlags": []}
            }),
        },
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle thread status");

    assert!(state.last_status_line.is_none());
}

#[test]
fn resolved_server_request_clears_waiting_on_approval_status() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    state.last_status_line = Some("waiting on approval".to_string());
    let mut output = Output::default();

    let handled = handle_status_update(
        "serverRequest/resolved",
        &serde_json::json!({
            "threadId": "thread-1",
            "requestId": "req-1"
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle resolved request");

    assert!(handled);
    assert!(state.last_status_line.is_none());
}

#[test]
fn background_terminal_tracking_survives_turn_reset_and_shows_recent_output() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();

    handle_status_update(
        "item/started",
        &serde_json::json!({
            "item": {
                "type": "commandExecution",
                "id": "cmd-1",
                "command": "python worker.py"
            }
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("start command item");
    handle_buffer_update(
        "item/commandExecution/outputDelta",
        &serde_json::json!({
            "itemId": "cmd-1",
            "delta": "booting\\nready\\n"
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("buffer output");
    handle_buffer_update(
        "item/commandExecution/terminalInteraction",
        &serde_json::json!({
            "itemId": "cmd-1",
            "processId": "proc-1",
            "stdin": ""
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("track background terminal");

    state.reset_turn_stream_state();

    assert_eq!(background_terminal_count(&state), 1);
    let rendered = render_background_terminals(&state);
    assert!(rendered.contains("python worker.py"));
    assert!(rendered.contains("proc-1"));
    assert!(rendered.contains("ready"));
}

#[test]
fn ready_status_mentions_blocking_prereqs_services_and_terminals() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-main".to_string());
    state
        .background_shells
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite shell");
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

    let rendered = render_prompt_status(&state);
    assert!(rendered.contains("blocked on 1 prerequisite shell"));
    assert!(rendered.contains("1 service untracked"));
    assert!(rendered.contains("1 terminal"));
    assert!(rendered.contains("/ps to view"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn background_task_rendering_includes_local_background_shell_jobs() {
    let state = crate::state::AppState::new(true, false);
    state
        .background_shells
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start background shell");

    let rendered = render_background_terminals(&state);
    assert!(rendered.contains("Local background shell jobs:"));
    assert!(rendered.contains("bg-1"));
    assert!(rendered.contains("sleep 0.4"));
    let _ = state.background_shells.terminate_all_running();
}

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
        "orchestration   main=1 deps_blocking=0 deps_sidecar=2 waits=0 sidecar_agents=1 exec_prereqs=0 exec_sidecars=1 exec_services=0 services_ready=0 services_booting=0 services_untracked=0 service_caps=0 service_cap_conflicts=0 cap_deps_missing=0 cap_deps_booting=0 cap_deps_ambiguous=0 agents_live=1 agents_cached=2"
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
        "background cls  prereqs=1 shell_sidecars=1 services=1 services_ready=0 services_booting=0 services_untracked=1 cap_deps_missing=0 cap_deps_booting=0 cap_deps_ambiguous=0 terminals=1"
    ));
    assert!(rendered.contains("next action     Main agent is blocked on 1 prerequisite shell."));
    let _ = state.background_shells.terminate_all_running();
}

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
    assert_eq!(parse_ps_filter(Some("clean")), None);
    assert_eq!(parse_ps_filter(Some("unknown")), None);
}

#[test]
fn clean_target_parser_accepts_scoped_cleanup_aliases() {
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

#[test]
fn ps_command_can_invoke_service_recipe_for_aliased_job() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);
            assert!(request.starts_with("GET /health HTTP/1.1\r\n"));
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            )
            .expect("write response");
        }
    });

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
                "protocol": "http",
                "endpoint": format!("http://{addr}"),
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
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
        "run dev.api health",
        &["run", "dev.api", "health"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("invoke service recipe");

    let rendered = state
        .background_shells
        .invoke_recipe_for_operator("bg-1", "health")
        .expect("invoke recipe directly after command path");
    assert!(rendered.contains("Action: http GET /health"));
    assert!(rendered.contains("HTTP/1.1 200 OK"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn ps_command_can_invoke_parameterized_service_recipe_for_aliased_job() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "recipes": [
                    {
                        "name": "say",
                        "description": "Write one message to the service shell",
                        "parameters": [
                            {
                                "name": "message",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "stdin",
                            "text": "{{message}}"
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
        r#"run dev.api say {"message":"hello from parameterized recipe"}"#,
        &[
            "run",
            "dev.api",
            "say",
            r#"{"message":"hello"#,
            "from",
            "parameterized",
            r#"recipe"}"#,
        ],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("invoke parameterized recipe");

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = state
            .background_shells
            .poll_job("bg-1", 0, 200)
            .expect("poll shell directly");
        if rendered.contains("hello from parameterized recipe") {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("hello from parameterized recipe"));
    let _ = state.background_shells.terminate_all_running();
}

#[test]
fn collab_wait_item_sets_waiting_on_agent_status() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();

    handle_status_update(
        "item/started",
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "wait-1",
                "tool": "wait",
                "status": "inProgress",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "running"
                    }
                }
            }
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle wait start");

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on agent thread-agent-1")
    );
}

#[test]
fn completing_one_wait_task_keeps_status_for_remaining_waits() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    for (call_id, agent_id) in [("wait-1", "thread-agent-1"), ("wait-2", "thread-agent-2")] {
        handle_status_update(
            "item/started",
            &json!({
                "item": {
                    "type": "collabAgentToolCall",
                    "id": call_id,
                    "tool": "wait",
                    "status": "inProgress",
                    "senderThreadId": "thread-main",
                    "receiverThreadIds": [agent_id],
                    "agentsStates": {
                        agent_id: {
                            "status": "running"
                        }
                    }
                }
            }),
            &cli,
            &mut state,
            &mut output,
        )
        .expect("start wait task");
    }

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on agents thread-agent-1, thread-agent-2")
    );

    render_item_completed(
        &cli,
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "wait-1",
                "tool": "wait",
                "status": "completed",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "completed",
                        "message": "done"
                    }
                }
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("complete first wait");

    assert_eq!(
        state.last_status_line.as_deref(),
        Some("waiting on agent thread-agent-2")
    );
}

#[test]
fn collab_agent_items_register_live_agent_tasks_and_cache_threads() {
    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();

    handle_status_update(
        "item/started",
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "call-1",
                "tool": "spawnAgent",
                "status": "inProgress",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "prompt": "Inspect auth flow and report risks",
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "running",
                        "message": "reviewing auth flow"
                    }
                }
            }
        }),
        &cli,
        &mut state,
        &mut output,
    )
    .expect("handle collab start");

    assert_eq!(state.live_agent_tasks.len(), 1);
    assert_eq!(state.cached_agent_threads.len(), 1);
    assert_eq!(state.cached_agent_threads[0].id, "thread-agent-1");
    assert_eq!(state.cached_agent_threads[0].status, "running");

    render_item_completed(
        &cli,
        &json!({
            "item": {
                "type": "collabAgentToolCall",
                "id": "call-1",
                "tool": "spawnAgent",
                "status": "completed",
                "senderThreadId": "thread-main",
                "receiverThreadIds": ["thread-agent-1"],
                "agentsStates": {
                    "thread-agent-1": {
                        "status": "completed",
                        "message": "done"
                    }
                }
            }
        }),
        &mut state,
        &mut output,
    )
    .expect("complete collab call");

    assert!(state.live_agent_tasks.is_empty());
    assert_eq!(state.cached_agent_threads[0].status, "completed");
    assert_eq!(state.cached_agent_threads[0].preview, "done");
}
