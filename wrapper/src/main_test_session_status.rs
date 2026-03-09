use crate::Cli;
use crate::background_terminals::background_terminal_count;
use crate::background_terminals::render_background_terminals;
use crate::dispatch_command_session_ps::handle_ps_command;
use crate::dispatch_command_session_ps::parse_clean_selection;
use crate::dispatch_command_session_ps::parse_clean_target;
use crate::dispatch_command_session_ps::parse_ps_capability_issue_filter;
use crate::dispatch_command_session_ps::parse_ps_dependency_filter;
use crate::dispatch_command_session_ps::parse_ps_dependency_selector;
use crate::dispatch_command_session_ps::parse_ps_filter;
use crate::dispatch_command_session_ps::parse_ps_focus_capability;
use crate::dispatch_command_session_ps::parse_ps_service_issue_filter;
use crate::dispatch_command_session_ps::parse_ps_service_selector;
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

#[path = "main_test_session_status_ps_orchestration.rs"]
mod ps_orchestration;
#[path = "main_test_session_status_ps_recipes.rs"]
mod ps_recipes;
#[path = "main_test_session_status_ps_services.rs"]
mod ps_services;

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
    assert!(rendered.contains(":ps to view"));
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
