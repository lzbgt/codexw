use std::io::BufRead;
use std::io::BufReader;
use std::process::Child;
use std::process::ChildStdin;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::terminal;

use super::Cli;

pub(crate) enum AppEvent {
    ServerLine(String),
    InputKey(InputKey),
    Tick,
    StdinClosed,
    ServerClosed,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum InputKey {
    Char(char),
    Esc,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
    Tab,
    Enter,
    CtrlJ,
    CtrlC,
    CtrlA,
    CtrlE,
    CtrlU,
    CtrlW,
}

pub(crate) struct StartMode {
    pub(crate) resume_thread_id: Option<String>,
    pub(crate) initial_prompt: Option<String>,
}

pub(crate) fn normalize_cli(mut cli: Cli) -> Cli {
    if cli.resume.is_none() && matches!(cli.prompt.first().map(String::as_str), Some("resume")) {
        if let Some(thread_id) = cli.prompt.get(1).cloned() {
            cli.resume = Some(thread_id);
            cli.prompt.drain(0..2);
        }
    }
    cli
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

pub(crate) fn start_stdout_thread(stdout: ChildStdout, tx: mpsc::Sender<AppEvent>) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(AppEvent::ServerClosed);
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
                    let _ = tx.send(AppEvent::ServerLine(trimmed));
                }
                Err(_) => {
                    let _ = tx.send(AppEvent::ServerClosed);
                    break;
                }
            }
        }
    });
}

pub(crate) fn start_stdin_thread(tx: mpsc::Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(Event::Key(key_event)) => {
                    if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                        continue;
                    }
                    if let Some(key) = map_key_event(key_event) {
                        let _ = tx.send(AppEvent::InputKey(key));
                    }
                }
                Ok(_) => {}
                Err(_) => {
                    let _ = tx.send(AppEvent::StdinClosed);
                    break;
                }
            }
        }
    });
}

pub(crate) fn start_tick_thread(tx: mpsc::Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(200));
            if tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });
}

fn map_key_event(key_event: KeyEvent) -> Option<InputKey> {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, _) => Some(InputKey::Esc),
        (KeyCode::Char('c'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlC)
        }
        (KeyCode::Char('j'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlJ)
        }
        (KeyCode::Char('a'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlA)
        }
        (KeyCode::Char('e'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlE)
        }
        (KeyCode::Char('u'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlU)
        }
        (KeyCode::Char('w'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputKey::CtrlW)
        }
        (KeyCode::Enter, _) => Some(InputKey::Enter),
        (KeyCode::Backspace, _) => Some(InputKey::Backspace),
        (KeyCode::Delete, _) => Some(InputKey::Delete),
        (KeyCode::Left, _) => Some(InputKey::Left),
        (KeyCode::Right, _) => Some(InputKey::Right),
        (KeyCode::Home, _) => Some(InputKey::Home),
        (KeyCode::End, _) => Some(InputKey::End),
        (KeyCode::Up, _) => Some(InputKey::Up),
        (KeyCode::Down, _) => Some(InputKey::Down),
        (KeyCode::Tab, _) => Some(InputKey::Tab),
        (KeyCode::Char(ch), modifiers)
            if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
        {
            Some(InputKey::Char(ch))
        }
        _ => None,
    }
}

pub(crate) struct RawModeGuard;

impl RawModeGuard {
    pub(crate) fn new() -> Result<Self> {
        terminal::enable_raw_mode().context("enable raw terminal mode")?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

pub(crate) fn shutdown_child(writer: ChildStdin, mut child: Child) -> Result<()> {
    drop(writer);
    let status = child.wait().context("wait for codex app-server exit")?;
    if !status.success() {
        eprintln!("[session] codex app-server exited with {status}");
    }
    Ok(())
}

pub(crate) fn effective_cwd(cli: &Cli) -> Result<String> {
    cli.cwd
        .as_deref()
        .map(|path| {
            std::fs::canonicalize(path)
                .with_context(|| format!("canonicalize cwd `{path}`"))
                .map(|value| value.to_string_lossy().to_string())
        })
        .transpose()?
        .map(Ok)
        .unwrap_or_else(|| std::env::current_dir().map(|p| p.to_string_lossy().to_string()))
        .context("resolve current working directory")
}
