use crate::Cli;
use crate::runtime_process::effective_cwd;
use crate::runtime_process::shutdown_child;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;
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
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
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
fn shutdown_child_terminates_process_that_ignores_stdin_close() {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("sleep 30")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn stubborn child");
    let writer = child.stdin.take().expect("child stdin");

    let started = Instant::now();
    shutdown_child(writer, child).expect("shutdown child");
    assert!(started.elapsed() < Duration::from_secs(5));
}
