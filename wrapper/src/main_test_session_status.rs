use crate::Cli;
use crate::prompt_state::render_prompt_status;
use crate::session_snapshot_overview::render_status_overview;
use crate::session_snapshot_runtime::render_status_runtime;
use crate::transcript_status_summary::summarize_thread_status_for_display;
use serde_json::json;

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

    state.reset_thread_context();

    assert!(state.command_output_buffers.is_empty());
    assert!(state.file_output_buffers.is_empty());
    assert!(state.process_output_buffers.is_empty());
    assert!(state.last_agent_message.is_none());
    assert!(state.last_turn_diff.is_none());
    assert!(state.last_status_line.is_none());
    assert!(!state.startup_resume_picker);
}
