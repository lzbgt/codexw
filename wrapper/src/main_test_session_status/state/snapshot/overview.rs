use super::*;

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
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
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
