use std::process::Child;
use std::process::Command;
use std::process::Stdio;

use serde_json::Value;

use crate::Cli;

pub(super) fn build_cli() -> Cli {
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

pub(super) fn spawn_sink_stdin() -> std::process::ChildStdin {
    Command::new("sh")
        .arg("-c")
        .arg("cat >/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sink")
        .stdin
        .take()
        .expect("stdin")
}

pub(super) fn spawn_recording_stdin() -> (
    tempfile::TempDir,
    Child,
    std::process::ChildStdin,
    std::path::PathBuf,
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("requests.jsonl");
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("cat > \"$1\"")
        .arg("sh")
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn recorder");
    let stdin = child.stdin.take().expect("stdin");
    (temp, child, stdin, path)
}

pub(super) fn read_recorded_requests(
    child: &mut Child,
    writer: std::process::ChildStdin,
    path: &std::path::Path,
) -> Vec<Value> {
    drop(writer);
    child.wait().expect("wait recorder");
    let contents = std::fs::read_to_string(path).expect("read requests");
    contents
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("parse request"))
        .collect()
}

pub(super) fn test_codex_home() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_home = temp.path().join("codex-home");
    (temp, codex_home)
}

pub(super) fn config_contents(codex_home: &std::path::Path) -> String {
    std::fs::read_to_string(codex_home.join("config.toml")).expect("read config")
}
