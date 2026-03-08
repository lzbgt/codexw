use crate::Cli;
use crate::session_prompt_status::render_prompt_status;
use crate::session_snapshot::render_status_snapshot;
use crate::transcript_summary::summarize_thread_status_for_display;
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
fn status_snapshot_includes_realtime_fields() {
    let mut state = crate::state::AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.realtime_active = true;
    state.realtime_session_id = Some("rt-1".to_string());
    state.realtime_prompt = Some("hello world".to_string());
    state.realtime_last_error = Some("bad gateway".to_string());
    let cli = crate::runtime::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
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
    let rendered = render_status_snapshot(&cli, "/tmp/project", &state);
    assert!(rendered.contains("realtime        true"));
    assert!(rendered.contains("realtime id     rt-1"));
    assert!(rendered.contains("realtime prompt hello world"));
    assert!(rendered.contains("realtime error  bad gateway"));
}
