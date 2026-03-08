use crate::Cli;
use crate::app::build_resume_command;
use crate::app::build_resume_hint_line;
use crate::commands_completion_render::quote_if_needed;
use crate::dispatch_command_utils::parse_feedback_args;
use crate::runtime_process::effective_cwd;
use crate::runtime_process::normalize_cli;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

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

fn current_dir_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &std::path::Path) -> Self {
        let original = std::env::current_dir().expect("capture current dir");
        std::env::set_current_dir(path).expect("change current dir");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).expect("restore current dir");
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

#[cfg(unix)]
#[test]
fn effective_cwd_canonicalizes_current_dir_without_explicit_flag() {
    let _lock = current_dir_lock().lock().expect("lock current dir");
    let workspace = tempdir().expect("tempdir");
    let real_dir = workspace.path().join("real");
    let link_dir = workspace.path().join("link");
    std::fs::create_dir(&real_dir).expect("create real dir");
    symlink(&real_dir, &link_dir).expect("create symlink");
    let _guard = CurrentDirGuard::change_to(&link_dir);

    let resolved = effective_cwd(&cli_with_defaults()).expect("resolve cwd");
    assert_eq!(
        PathBuf::from(resolved),
        real_dir.canonicalize().expect("canonical real dir")
    );
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
