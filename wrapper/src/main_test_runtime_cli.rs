use crate::Cli;
use crate::commands_completion_render::quote_if_needed;
use crate::dispatch_command_utils::parse_feedback_args;
use crate::runtime_process::normalize_cli;

#[test]
fn normalize_cli_supports_codex_style_resume_startup() {
    let cli = normalize_cli(Cli {
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
        prompt: vec![
            "resume".to_string(),
            "thread-123".to_string(),
            "continue".to_string(),
            "work".to_string(),
        ],
    });
    assert_eq!(cli.resume.as_deref(), Some("thread-123"));
    assert_eq!(cli.prompt, vec!["continue".to_string(), "work".to_string()]);
}

#[test]
fn feedback_args_parse_category_reason_and_logs() {
    let parsed = parse_feedback_args(&[
        "bug".to_string(),
        "command".to_string(),
        "output".to_string(),
        "was".to_string(),
        "wrong".to_string(),
        "--logs".to_string(),
    ])
    .expect("expected feedback args to parse");
    assert_eq!(parsed.classification, "bug");
    assert_eq!(parsed.reason.as_deref(), Some("command output was wrong"));
    assert!(parsed.include_logs);
}

#[test]
fn feedback_args_accept_aliases() {
    let parsed =
        parse_feedback_args(&["good".to_string()]).expect("expected feedback args to parse");
    assert_eq!(parsed.classification, "good_result");
    assert_eq!(parsed.reason, None);
    assert!(!parsed.include_logs);
}

#[test]
fn quote_if_needed_leaves_simple_paths_unquoted() {
    assert_eq!(quote_if_needed("src/main.rs"), "src/main.rs");
    assert_eq!(
        quote_if_needed("path with spaces.rs"),
        "\"path with spaces.rs\""
    );
}
