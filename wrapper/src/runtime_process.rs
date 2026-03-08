use std::process::Child;
use std::process::ChildStdin;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;

use super::Cli;

pub(crate) enum StartupThreadAction {
    Create,
    Resume(String),
    ResumePicker,
}

pub(crate) struct StartMode {
    pub(crate) thread_action: StartupThreadAction,
    pub(crate) initial_prompt: Option<String>,
}

pub(crate) fn normalize_cli(mut cli: Cli) -> Cli {
    if cli.resume.is_none()
        && !cli.resume_picker
        && matches!(cli.prompt.first().map(String::as_str), Some("resume"))
    {
        if let Some(thread_id) = cli.prompt.get(1).cloned() {
            cli.resume = Some(thread_id);
            cli.prompt.drain(0..2);
        } else {
            cli.resume_picker = true;
            cli.prompt.drain(0..1);
        }
    }
    cli
}

pub(crate) fn build_start_mode(cli: &Cli, initial_prompt: Option<String>) -> StartMode {
    let thread_action = if cli.resume_picker {
        StartupThreadAction::ResumePicker
    } else if let Some(thread_id) = cli.resume.clone() {
        StartupThreadAction::Resume(thread_id)
    } else {
        StartupThreadAction::Create
    };
    StartMode {
        thread_action,
        initial_prompt,
    }
}

pub(crate) fn spawn_server(cli: &Cli, resolved_cwd: &str) -> Result<Child> {
    let mut cmd = Command::new(&cli.codex_bin);
    for kv in &cli.config_overrides {
        cmd.arg("--config").arg(kv);
    }
    for feature in &cli.enable_features {
        cmd.arg("--enable").arg(feature);
    }
    for feature in &cli.disable_features {
        cmd.arg("--disable").arg(feature);
    }
    cmd.arg("app-server")
        .arg("--listen")
        .arg("stdio://")
        .current_dir(resolved_cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    inherit_proxy_env(&mut cmd);
    cmd.spawn()
        .with_context(|| format!("failed to start `{}` app-server", cli.codex_bin))
}

fn inherit_proxy_env(cmd: &mut Command) {
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
        "NO_PROXY",
        "no_proxy",
    ] {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
}

pub(crate) fn shutdown_child(writer: ChildStdin, mut child: Child) -> Result<()> {
    const SHUTDOWN_WAIT: Duration = Duration::from_millis(750);
    const POLL_INTERVAL: Duration = Duration::from_millis(25);

    drop(writer);
    let deadline = Instant::now() + SHUTDOWN_WAIT;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .context("poll codex app-server exit during shutdown")?
        {
            break status;
        }
        if Instant::now() >= deadline {
            match child.kill() {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => {}
                Err(err) => {
                    return Err(err).context("terminate codex app-server after shutdown timeout");
                }
            }
            break child
                .wait()
                .context("wait for codex app-server exit after terminate")?;
        }
        thread::sleep(POLL_INTERVAL);
    };
    if !status.success() {
        eprintln!("[session] codex app-server exited with {status}");
    }
    Ok(())
}

pub(crate) fn effective_cwd(cli: &Cli) -> Result<String> {
    let cwd = match cli.cwd.as_deref() {
        Some(path) => std::path::PathBuf::from(path),
        None => std::env::current_dir().context("resolve current working directory")?,
    };
    std::fs::canonicalize(&cwd)
        .with_context(|| format!("canonicalize cwd `{}`", cwd.to_string_lossy()))
        .map(|value| value.to_string_lossy().to_string())
}
