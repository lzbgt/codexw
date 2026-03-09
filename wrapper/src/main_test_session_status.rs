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
#[path = "main_test_session_status/runtime.rs"]
mod runtime;

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
