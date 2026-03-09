use crate::Cli;
use crate::app::build_resume_command;
use crate::app::build_resume_hint_line;
use crate::commands_completion_render::quote_if_needed;
use crate::dispatch_command_utils::parse_feedback_args;
use crate::runtime_process::normalize_cli;

fn cli_with_defaults() -> Cli {
    Cli {
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
    }
}

#[test]
fn normalize_cli_supports_codex_style_resume_startup() {
    let cli = normalize_cli(Cli {
        prompt: vec![
            "resume".to_string(),
            "thread-123".to_string(),
            "continue".to_string(),
            "work".to_string(),
        ],
        ..cli_with_defaults()
    });
    assert_eq!(cli.resume.as_deref(), Some("thread-123"));
    assert!(!cli.resume_picker);
    assert_eq!(cli.prompt, vec!["continue".to_string(), "work".to_string()]);
}

#[test]
fn normalize_cli_supports_codex_style_resume_picker_startup() {
    let cli = normalize_cli(Cli {
        prompt: vec!["resume".to_string()],
        ..cli_with_defaults()
    });
    assert_eq!(cli.resume, None);
    assert!(cli.resume_picker);
    assert!(cli.prompt.is_empty());
}

#[test]
fn normalize_cli_disables_responses_websockets_by_default() {
    let cli = normalize_cli(cli_with_defaults());
    assert!(
        cli.disable_features
            .iter()
            .any(|value| value == "responses_websockets")
    );
    assert!(
        cli.disable_features
            .iter()
            .any(|value| value == "responses_websockets_v2")
    );
}

#[test]
fn normalize_cli_respects_explicit_websocket_enable_flags() {
    let cli = normalize_cli(Cli {
        enable_features: vec!["responses_websockets".to_string()],
        ..cli_with_defaults()
    });
    assert!(
        cli.enable_features
            .iter()
            .any(|value| value == "responses_websockets")
    );
    assert!(
        !cli.disable_features
            .iter()
            .any(|value| value == "responses_websockets")
    );
    assert!(
        cli.disable_features
            .iter()
            .any(|value| value == "responses_websockets_v2")
    );
}

#[test]
fn parse_cli_accepts_cwd_after_resume_for_picker_startup() {
    let cli = crate::parse_cli_from(["codexw", "resume", "--cwd", "/tmp/project"])
        .expect("parse reordered resume picker");
    assert_eq!(cli.cwd.as_deref(), Some("/tmp/project"));
    assert_eq!(cli.resume, None);
    assert!(cli.resume_picker);
    assert!(cli.prompt.is_empty());
}

#[test]
fn parse_cli_accepts_cwd_after_resume_for_explicit_thread() {
    let cli = crate::parse_cli_from([
        "codexw",
        "resume",
        "thread-123",
        "--cwd=/tmp/project",
        "continue",
        "work",
    ])
    .expect("parse reordered resume thread");
    assert_eq!(cli.cwd.as_deref(), Some("/tmp/project"));
    assert_eq!(cli.resume.as_deref(), Some("thread-123"));
    assert!(!cli.resume_picker);
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

#[test]
fn build_resume_command_includes_cwd_and_thread_id() {
    assert_eq!(
        build_resume_command("codexw", "/tmp/work tree", "thread-123"),
        "codexw --cwd \"/tmp/work tree\" resume thread-123"
    );
}

#[test]
fn build_resume_hint_line_includes_full_resume_command() {
    assert_eq!(
        build_resume_hint_line("codexw", "/tmp/work tree", Some("thread-123")).as_deref(),
        Some("[session] resume with: codexw --cwd \"/tmp/work tree\" resume thread-123")
    );
    assert_eq!(
        build_resume_hint_line("codexw", "/tmp/work tree", None),
        None
    );
}
