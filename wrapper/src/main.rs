mod commands;
mod editor;
mod events;
mod history;
mod input;
mod interaction;
mod output;
mod prompt;
mod render;
mod requests;
mod rpc;
mod session;
mod views;

use std::collections::HashMap;
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
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use base64::Engine;
use clap::ArgAction;
use clap::Parser;
use commands::builtin_command_names;
use commands::builtin_help_lines;
use commands::longest_common_prefix;
use commands::quote_if_needed;
use commands::try_complete_slash_command;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::terminal;
use editor::EditorEvent;
use editor::LineEditor;
use events::process_server_line;
use input::AppCatalogEntry;
use input::PluginCatalogEntry;
use input::SkillCatalogEntry;
use input::build_turn_input;
use interaction::handle_tab_completion;
use interaction::handle_user_input;
use interaction::join_prompt;
use interaction::prompt_accepts_input;
use interaction::update_prompt;
use output::Output;
use prompt::build_continue_prompt;
use prompt::parse_auto_mode_stop;
use requests::PendingRequest;
use requests::send_clean_background_terminals;
use requests::send_command_exec;
use requests::send_command_exec_terminate;
use requests::send_feedback_upload;
use requests::send_fuzzy_file_search;
use requests::send_initialize;
use requests::send_list_threads;
use requests::send_load_collaboration_modes;
use requests::send_load_config;
use requests::send_load_experimental_features;
use requests::send_load_mcp_servers;
use requests::send_load_models;
use requests::send_logout_account;
use requests::send_start_review;
use requests::send_thread_compact;
use requests::send_thread_fork;
use requests::send_thread_realtime_append_text;
use requests::send_thread_realtime_start;
use requests::send_thread_realtime_stop;
use requests::send_thread_rename;
use requests::send_thread_resume;
use requests::send_thread_start;
use requests::send_turn_interrupt;
use requests::send_turn_start;
use requests::send_turn_steer;
use rpc::RequestId;
use serde_json::Value;
use serde_json::json;
use session::CollaborationModeAction;
use session::CollaborationModePreset;
use session::ModelCatalogEntry;
use session::ModelsAction;
use session::apply_collaboration_mode_action;
use session::apply_models_action;
use session::apply_personality_selection;
use session::extract_collaboration_mode_presets;
use session::render_personality_options;
use session::render_prompt_status;
use session::render_realtime_item;
use session::render_realtime_status;
use session::render_status_snapshot;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Codex app-server inline terminal client with auto-continue"
)]
struct Cli {
    #[arg(long, default_value = "codex")]
    codex_bin: String,

    #[arg(short = 'c', long = "config", value_name = "key=value", action = ArgAction::Append)]
    config_overrides: Vec<String>,

    #[arg(long = "enable", value_name = "FEATURE", action = ArgAction::Append)]
    enable_features: Vec<String>,

    #[arg(long = "disable", value_name = "FEATURE", action = ArgAction::Append)]
    disable_features: Vec<String>,

    #[arg(long)]
    resume: Option<String>,

    #[arg(long)]
    cwd: Option<String>,

    #[arg(long)]
    model: Option<String>,

    #[arg(long)]
    model_provider: Option<String>,

    #[arg(long, default_value_t = true)]
    auto_continue: bool,

    #[arg(long, default_value_t = false)]
    verbose_events: bool,

    #[arg(long, default_value_t = true)]
    verbose_thinking: bool,

    #[arg(long, default_value_t = false)]
    raw_json: bool,

    #[arg(long, default_value_t = false)]
    no_experimental_api: bool,

    #[arg(long, default_value_t = false)]
    yolo: bool,

    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

enum AppEvent {
    ServerLine(String),
    InputKey(InputKey),
    Tick,
    StdinClosed,
    ServerClosed,
}

#[derive(Debug, Clone, Copy)]
enum InputKey {
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

#[derive(Default)]
struct ProcessOutputBuffer {
    stdout: String,
    stderr: String,
}

struct AppState {
    thread_id: Option<String>,
    active_turn_id: Option<String>,
    active_exec_process_id: Option<String>,
    realtime_active: bool,
    realtime_session_id: Option<String>,
    realtime_last_error: Option<String>,
    realtime_started_at: Option<Instant>,
    realtime_prompt: Option<String>,
    pending_thread_switch: bool,
    turn_running: bool,
    activity_started_at: Option<Instant>,
    started_turn_count: u64,
    completed_turn_count: u64,
    auto_continue: bool,
    objective: Option<String>,
    last_agent_message: Option<String>,
    last_turn_diff: Option<String>,
    last_token_usage: Option<Value>,
    account_info: Option<Value>,
    rate_limits: Option<Value>,
    command_output_buffers: HashMap<String, String>,
    file_output_buffers: HashMap<String, String>,
    process_output_buffers: HashMap<String, ProcessOutputBuffer>,
    pending_local_images: Vec<String>,
    pending_remote_images: Vec<String>,
    active_personality: Option<String>,
    apps: Vec<AppCatalogEntry>,
    plugins: Vec<PluginCatalogEntry>,
    skills: Vec<SkillCatalogEntry>,
    models: Vec<ModelCatalogEntry>,
    collaboration_modes: Vec<CollaborationModePreset>,
    active_collaboration_mode: Option<CollaborationModePreset>,
    last_listed_thread_ids: Vec<String>,
    last_file_search_paths: Vec<String>,
    last_status_line: Option<String>,
    raw_json: bool,
    pending: HashMap<RequestId, PendingRequest>,
    next_request_id: i64,
}

impl AppState {
    fn new(auto_continue: bool, raw_json: bool) -> Self {
        Self {
            thread_id: None,
            active_turn_id: None,
            active_exec_process_id: None,
            realtime_active: false,
            realtime_session_id: None,
            realtime_last_error: None,
            realtime_started_at: None,
            realtime_prompt: None,
            pending_thread_switch: false,
            turn_running: false,
            activity_started_at: None,
            started_turn_count: 0,
            completed_turn_count: 0,
            auto_continue,
            objective: None,
            last_agent_message: None,
            last_turn_diff: None,
            last_token_usage: None,
            account_info: None,
            rate_limits: None,
            command_output_buffers: HashMap::new(),
            file_output_buffers: HashMap::new(),
            process_output_buffers: HashMap::new(),
            pending_local_images: Vec::new(),
            pending_remote_images: Vec::new(),
            active_personality: None,
            apps: Vec::new(),
            plugins: Vec::new(),
            skills: Vec::new(),
            models: Vec::new(),
            collaboration_modes: Vec::new(),
            active_collaboration_mode: None,
            last_listed_thread_ids: Vec::new(),
            last_file_search_paths: Vec::new(),
            last_status_line: None,
            raw_json,
            pending: HashMap::new(),
            next_request_id: 1,
        }
    }

    fn next_request_id(&mut self) -> RequestId {
        let id = self.next_request_id;
        self.next_request_id += 1;
        RequestId::Integer(id)
    }

    fn reset_turn_stream_state(&mut self) {
        self.command_output_buffers.clear();
        self.file_output_buffers.clear();
        self.last_agent_message = None;
        self.last_turn_diff = None;
        self.last_status_line = None;
    }

    fn reset_thread_context(&mut self) {
        self.active_turn_id = None;
        self.active_exec_process_id = None;
        self.realtime_active = false;
        self.realtime_session_id = None;
        self.realtime_last_error = None;
        self.realtime_started_at = None;
        self.realtime_prompt = None;
        self.turn_running = false;
        self.activity_started_at = None;
        self.started_turn_count = 0;
        self.completed_turn_count = 0;
        self.objective = None;
        self.last_agent_message = None;
        self.last_turn_diff = None;
        self.last_token_usage = None;
        self.last_status_line = None;
        self.active_personality = None;
        self.active_collaboration_mode = None;
    }

    fn take_pending_attachments(&mut self) -> (Vec<String>, Vec<String>) {
        let local = std::mem::take(&mut self.pending_local_images);
        let remote = std::mem::take(&mut self.pending_remote_images);
        (local, remote)
    }
}

fn main() -> Result<()> {
    let cli = normalize_cli(Cli::parse());
    let initial_prompt = join_prompt(&cli.prompt);
    let resolved_cwd = effective_cwd(&cli)?;
    let _raw_mode = RawModeGuard::new()?;

    let mut child = spawn_server(&cli, &resolved_cwd)?;
    let stdin = child
        .stdin
        .take()
        .context("codex app-server stdin unavailable")?;
    let stdout = child
        .stdout
        .take()
        .context("codex app-server stdout unavailable")?;

    let (tx, rx) = mpsc::channel::<AppEvent>();
    start_stdout_thread(stdout, tx.clone());
    start_stdin_thread(tx.clone());
    start_tick_thread(tx.clone());

    let mut output = Output::default();
    let mut writer = stdin;
    let mut state = AppState::new(cli.auto_continue, cli.raw_json);
    let mut editor = LineEditor::default();

    output.line_stderr("[session] connecting to codex app-server")?;
    send_initialize(&mut writer, &mut state, &cli, !cli.no_experimental_api)?;

    let mut start_after_initialize = Some(StartMode {
        resume_thread_id: cli.resume.clone(),
        initial_prompt,
    });

    loop {
        update_prompt(&mut output, &state, &editor)?;
        match rx.recv() {
            Ok(AppEvent::ServerLine(line)) => {
                process_server_line(
                    line,
                    &cli,
                    &resolved_cwd,
                    &mut state,
                    &mut output,
                    &mut writer,
                    &mut start_after_initialize,
                )?;
            }
            Ok(AppEvent::InputKey(key)) => match key {
                InputKey::Char(ch) => {
                    if prompt_accepts_input(&state) {
                        editor.insert_char(ch);
                    }
                }
                InputKey::Esc => {
                    if state.turn_running {
                        if let Some(turn_id) = state.active_turn_id.clone() {
                            let current_thread_id = thread_id(&state)?.to_string();
                            output.line_stderr("[interrupt] interrupting active turn")?;
                            send_turn_interrupt(
                                &mut writer,
                                &mut state,
                                current_thread_id,
                                turn_id,
                            )?;
                        } else {
                            output.line_stderr("[session] no active turn id; exiting")?;
                            break;
                        }
                    } else if let Some(process_id) = state.active_exec_process_id.clone() {
                        output.line_stderr("[interrupt] terminating active local command")?;
                        send_command_exec_terminate(&mut writer, &mut state, process_id)?;
                    } else if prompt_accepts_input(&state) {
                        editor.clear();
                    }
                }
                InputKey::Backspace => {
                    if prompt_accepts_input(&state) {
                        editor.backspace();
                    }
                }
                InputKey::Delete => {
                    if prompt_accepts_input(&state) {
                        editor.delete();
                    }
                }
                InputKey::Left => {
                    if prompt_accepts_input(&state) {
                        editor.move_left();
                    }
                }
                InputKey::Right => {
                    if prompt_accepts_input(&state) {
                        editor.move_right();
                    }
                }
                InputKey::Home => {
                    if prompt_accepts_input(&state) {
                        editor.move_home();
                    }
                }
                InputKey::End => {
                    if prompt_accepts_input(&state) {
                        editor.move_end();
                    }
                }
                InputKey::Up => {
                    if prompt_accepts_input(&state) {
                        editor.history_prev();
                    }
                }
                InputKey::Down => {
                    if prompt_accepts_input(&state) {
                        editor.history_next();
                    }
                }
                InputKey::Tab => {
                    if prompt_accepts_input(&state) {
                        handle_tab_completion(&mut editor, &state, &resolved_cwd, &mut output)?;
                    }
                }
                InputKey::CtrlA => {
                    if prompt_accepts_input(&state) {
                        editor.move_home();
                    }
                }
                InputKey::CtrlE => {
                    if prompt_accepts_input(&state) {
                        editor.move_end();
                    }
                }
                InputKey::CtrlU => {
                    if prompt_accepts_input(&state) {
                        editor.clear_to_start();
                    }
                }
                InputKey::CtrlW => {
                    if prompt_accepts_input(&state) {
                        editor.delete_prev_word();
                    }
                }
                InputKey::CtrlC => {
                    if state.turn_running {
                        editor.clear();
                        if let Some(turn_id) = state.active_turn_id.clone() {
                            let current_thread_id = thread_id(&state)?.to_string();
                            output.line_stderr("[interrupt] interrupting active turn")?;
                            send_turn_interrupt(
                                &mut writer,
                                &mut state,
                                current_thread_id,
                                turn_id,
                            )?;
                        } else {
                            output.line_stderr("[session] no active turn id; exiting")?;
                            break;
                        }
                    } else if let Some(process_id) = state.active_exec_process_id.clone() {
                        editor.clear();
                        output.line_stderr("[interrupt] terminating active local command")?;
                        send_command_exec_terminate(&mut writer, &mut state, process_id)?;
                    } else if matches!(editor.ctrl_c(), EditorEvent::CtrlC) {
                        output.line_stderr("[session] exiting on Ctrl-C")?;
                        break;
                    }
                }
                InputKey::Enter => match editor.submit() {
                    EditorEvent::Submit(line) => {
                        output.commit_prompt(&line)?;
                        if !handle_user_input(
                            line,
                            &cli,
                            &resolved_cwd,
                            &mut state,
                            &mut editor,
                            &mut output,
                            &mut writer,
                        )? {
                            break;
                        }
                    }
                    EditorEvent::CtrlC | EditorEvent::Noop => {}
                },
                InputKey::CtrlJ => {
                    if prompt_accepts_input(&state) {
                        editor.insert_newline();
                    }
                }
            },
            Ok(AppEvent::Tick) => {}
            Ok(AppEvent::StdinClosed) => {
                output.line_stderr("[session] stdin closed; exiting")?;
                break;
            }
            Ok(AppEvent::ServerClosed) => {
                output.line_stderr("[session] codex app-server exited")?;
                break;
            }
            Err(_) => break,
        }
    }

    shutdown_child(writer, child)?;
    Ok(())
}

fn normalize_cli(mut cli: Cli) -> Cli {
    if cli.resume.is_none() && matches!(cli.prompt.first().map(String::as_str), Some("resume")) {
        if let Some(thread_id) = cli.prompt.get(1).cloned() {
            cli.resume = Some(thread_id);
            cli.prompt.drain(0..2);
        }
    }
    cli
}

struct StartMode {
    resume_thread_id: Option<String>,
    initial_prompt: Option<String>,
}

fn spawn_server(cli: &Cli, resolved_cwd: &str) -> Result<Child> {
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

fn start_stdout_thread(stdout: ChildStdout, tx: mpsc::Sender<AppEvent>) {
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

fn start_stdin_thread(tx: mpsc::Sender<AppEvent>) {
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

fn start_tick_thread(tx: mpsc::Sender<AppEvent>) {
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

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode().context("enable raw terminal mode")?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn approval_policy(cli: &Cli) -> &'static str {
    let _ = cli;
    "never"
}

fn thread_sandbox_mode(cli: &Cli) -> &'static str {
    let _ = cli;
    "danger-full-access"
}

fn turn_sandbox_policy(cli: &Cli) -> Value {
    let _ = cli;
    json!({"type": "dangerFullAccess"})
}

fn reasoning_summary(cli: &Cli) -> &'static str {
    if cli.verbose_thinking {
        "detailed"
    } else {
        "auto"
    }
}

fn choose_command_approval_decision(params: &Value, yolo: bool) -> Value {
    let _ = yolo;
    if let Some(decisions) = params.get("availableDecisions").and_then(Value::as_array) {
        return choose_first_allowed_decision(decisions).unwrap_or_else(|| decisions[0].clone());
    }
    json!("accept")
}

fn choose_first_allowed_decision(decisions: &[Value]) -> Option<Value> {
    for preferred in [
        "acceptForSession",
        "acceptWithExecpolicyAmendment",
        "applyNetworkPolicyAmendment",
        "accept",
    ] {
        if let Some(found) = decisions
            .iter()
            .find(|decision| decision.as_str() == Some(preferred))
        {
            return Some(found.clone());
        }
    }
    None
}

fn shutdown_child(writer: ChildStdin, mut child: Child) -> Result<()> {
    drop(writer);
    if child.try_wait()?.is_none() {
        child.kill().context("kill codex app-server child")?;
        let _ = child.wait();
    }
    Ok(())
}

fn thread_id(state: &AppState) -> Result<&str> {
    state
        .thread_id
        .as_deref()
        .context("no active thread; wait for initialization or use :new")
}

fn effective_cwd(cli: &Cli) -> Result<String> {
    match cli.cwd.as_ref() {
        Some(cwd) => Ok(cwd.clone()),
        None => std::env::current_dir()
            .context("resolve current working directory")?
            .to_str()
            .map(ToOwned::to_owned)
            .context("current working directory is not valid UTF-8"),
    }
}

fn get_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn summarize_text(text: &str) -> String {
    const LIMIT: usize = 120;
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= LIMIT {
        single_line
    } else {
        let truncated = single_line
            .chars()
            .take(LIMIT.saturating_sub(1))
            .collect::<String>();
        format!("{truncated}…")
    }
}

fn parse_apps_list(result: &Value) -> Vec<AppCatalogEntry> {
    result
        .get("data")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    Some(AppCatalogEntry {
                        id: get_string(entry, &["id"])?.to_string(),
                        name: get_string(entry, &["name"])?.to_string(),
                        slug: app_slug(get_string(entry, &["name"])?),
                        enabled: entry
                            .get("isEnabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(true),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_skills_list(result: &Value, resolved_cwd: &str) -> Vec<SkillCatalogEntry> {
    result
        .get("data")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .find(|entry| get_string(entry, &["cwd"]) == Some(resolved_cwd))
                .and_then(|entry| entry.get("skills"))
                .and_then(Value::as_array)
                .map(|skills| {
                    skills
                        .iter()
                        .filter_map(|skill| {
                            Some(SkillCatalogEntry {
                                name: get_string(skill, &["name"])?.to_ascii_lowercase(),
                                path: get_string(skill, &["path"])?.to_string(),
                                enabled: skill
                                    .get("enabled")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(true),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn app_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn emit_status_line(_output: &mut Output, state: &mut AppState, line: String) -> Result<()> {
    if state.last_status_line.as_deref() == Some(line.as_str()) {
        return Ok(());
    }
    state.last_status_line = Some(line);
    Ok(())
}

fn buffer_item_delta(buffers: &mut HashMap<String, String>, params: &Value) {
    let Some(item_id) = get_string(params, &["itemId"]) else {
        return;
    };
    let Some(delta) = get_string(params, &["delta"]) else {
        return;
    };
    buffers
        .entry(item_id.to_string())
        .and_modify(|existing| existing.push_str(delta))
        .or_insert_with(|| delta.to_string());
}

fn buffer_process_delta(buffers: &mut HashMap<String, ProcessOutputBuffer>, params: &Value) {
    let Some(process_id) = get_string(params, &["processId"]) else {
        return;
    };
    let Some(encoded) = get_string(params, &["deltaBase64"]) else {
        return;
    };
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return;
    };
    let text = String::from_utf8_lossy(&decoded);
    let stream = get_string(params, &["stream"]).unwrap_or("stdout");
    let buffer = buffers.entry(process_id.to_string()).or_default();
    match stream {
        "stderr" => buffer.stderr.push_str(&text),
        _ => buffer.stdout.push_str(&text),
    }
}

fn canonicalize_or_keep(path: &str) -> String {
    std::fs::canonicalize(path)
        .ok()
        .and_then(|value| value.to_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| path.to_string())
}

fn shell_program() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use super::Cli;
    use super::CollaborationModePreset;
    use super::builtin_command_names;
    use super::builtin_help_lines;
    use super::choose_command_approval_decision;
    use super::extract_collaboration_mode_presets;
    use super::normalize_cli;
    use super::quote_if_needed;
    use super::render_personality_options;
    use super::render_prompt_status;
    use super::render_realtime_item;
    use super::render_status_snapshot;
    use super::try_complete_slash_command;
    use crate::commands::render_slash_completion_candidates;
    use crate::editor::LineEditor;
    use crate::events::params_auto_approval_result;
    use crate::history::latest_conversation_history_items;
    use crate::history::seed_resumed_state_from_turns;
    use crate::input::AppCatalogEntry;
    use crate::interaction::is_builtin_command;
    use crate::interaction::parse_feedback_args;
    use crate::interaction::prompt_accepts_input;
    use crate::interaction::prompt_is_visible;
    use crate::interaction::try_complete_file_token;
    use crate::session::extract_models;
    use crate::session::render_collaboration_modes;
    use crate::session::summarize_active_collaboration_mode;
    use crate::session::summarize_active_personality;
    use crate::views::build_tool_user_input_response;
    use crate::views::extract_file_search_paths;
    use crate::views::extract_thread_ids;
    use crate::views::render_apps_list;
    use crate::views::render_experimental_features_list;
    use crate::views::render_fuzzy_file_search_results;
    use crate::views::render_models_list;
    use crate::views::render_rate_limit_lines;
    use crate::views::render_reasoning_item;
    use crate::views::render_thread_list;
    use crate::views::summarize_terminal_interaction;
    use crate::views::summarize_thread_status_for_display;
    use serde_json::Value;
    use serde_json::json;

    #[test]
    fn yolo_prefers_first_available_command_approval_decision() {
        let params = json!({
            "availableDecisions": [
                "acceptForSession",
                "accept"
            ]
        });
        assert_eq!(
            choose_command_approval_decision(&params, true),
            json!("acceptForSession")
        );
    }

    #[test]
    fn command_approval_defaults_to_accept() {
        assert_eq!(
            choose_command_approval_decision(&json!({}), false),
            json!("accept")
        );
    }

    #[test]
    fn approval_prefers_allow_decisions_over_first_entry() {
        let params = json!({
            "availableDecisions": [
                "decline",
                "accept",
                "cancel"
            ]
        });
        assert_eq!(
            choose_command_approval_decision(&params, false),
            json!("accept")
        );
    }

    #[test]
    fn generic_approval_prefers_session_accept_when_available() {
        let params = json!({
            "availableDecisions": [
                "decline",
                "acceptForSession",
                "accept"
            ]
        });
        assert_eq!(
            params_auto_approval_result(&params),
            json!({"decision": "acceptForSession"})
        );
    }

    #[test]
    fn slash_aliases_are_treated_as_builtin_commands() {
        assert!(is_builtin_command("status"));
        assert!(is_builtin_command("statusline"));
        assert!(is_builtin_command("resume thread-1"));
        assert!(is_builtin_command("apps"));
        assert!(is_builtin_command("skills"));
        assert!(is_builtin_command("models"));
        assert!(is_builtin_command("settings"));
        assert!(is_builtin_command("compact"));
        assert!(is_builtin_command("review current changes"));
        assert!(is_builtin_command("permissions"));
        assert!(is_builtin_command("feedback bug something broke"));
        assert!(is_builtin_command("logout"));
        assert!(is_builtin_command("mcp"));
        assert!(is_builtin_command("threads"));
        assert!(is_builtin_command("mention foo"));
        assert!(is_builtin_command("diff"));
        assert!(!is_builtin_command("unknown-command"));
    }

    #[test]
    fn tool_user_input_defaults_to_first_option() {
        let response = build_tool_user_input_response(&json!({
            "questions": [
                {
                    "id": "confirm_path",
                    "options": [
                        {"label": "yes", "description": "Proceed"},
                        {"label": "no", "description": "Stop"}
                    ]
                }
            ]
        }));
        assert_eq!(
            response,
            json!({
                "answers": {
                    "confirm_path": { "answers": ["yes"] }
                }
            })
        );
    }

    #[test]
    fn reasoning_prefers_summary_blocks() {
        let rendered = render_reasoning_item(&json!({
            "summary": ["First block", "Second block"],
            "content": ["raw detail"]
        }));
        assert_eq!(rendered, "First block\n\nSecond block");
    }

    #[test]
    fn empty_terminal_interaction_is_suppressed() {
        assert_eq!(
            summarize_terminal_interaction(&json!({
                "processId": "123",
                "stdin": ""
            })),
            None
        );
    }

    #[test]
    fn terminal_interaction_only_surfaces_meaningful_stdin() {
        assert_eq!(
            summarize_terminal_interaction(&json!({
                "processId": "123",
                "stdin": "yes\n"
            })),
            Some("process=123 stdin=yes".to_string())
        );
    }

    #[test]
    fn tab_completes_unique_slash_command() {
        let mut editor = LineEditor::default();
        for ch in "/di".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();
        assert!(try_complete_slash_command(&mut editor, &buffer, cursor).is_some());
        assert_eq!(editor.buffer(), "/diff ");
    }

    #[test]
    fn ambiguous_slash_completion_lists_candidates() {
        let mut editor = LineEditor::default();
        for ch in "/re".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();
        let result = try_complete_slash_command(&mut editor, &buffer, cursor)
            .expect("expected slash completion result");
        let rendered = result.rendered_candidates.expect("expected candidate list");
        assert_eq!(editor.buffer(), "/re");
        assert!(rendered.contains("/resume"));
        assert!(rendered.contains("/review"));
    }

    #[test]
    fn fuzzy_slash_completion_lists_candidates() {
        let mut editor = LineEditor::default();
        for ch in "/ac".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();
        let result = try_complete_slash_command(&mut editor, &buffer, cursor)
            .expect("expected slash completion result");
        let rendered = result.rendered_candidates.expect("expected candidate list");
        assert_eq!(editor.buffer(), "/ac");
        assert!(rendered.contains("/feedback"));
        assert!(rendered.contains("Fuzzy matches for /ac:"));
    }

    #[test]
    fn tab_completes_unique_file_token() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file_path = temp.path().join("src").join("main.rs");
        std::fs::create_dir_all(file_path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&file_path, "fn main() {}\n").expect("write");

        let mut editor = LineEditor::default();
        for ch in "@src/ma".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();

        let result = try_complete_file_token(
            &mut editor,
            &buffer,
            cursor,
            temp.path().to_str().expect("utf8"),
        )
        .expect("complete")
        .expect("some result");

        assert!(result.rendered_candidates.is_none());
        assert_eq!(editor.buffer(), "src/main.rs ");
    }

    #[test]
    fn tab_lists_ambiguous_file_completions() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("alpha.txt"), "a").expect("write alpha");
        std::fs::write(temp.path().join("alpine.txt"), "b").expect("write alpine");

        let mut editor = LineEditor::default();
        for ch in "@al".chars() {
            editor.insert_char(ch);
        }
        let buffer = editor.buffer().to_string();
        let cursor = editor.cursor_byte_index();

        let result = try_complete_file_token(
            &mut editor,
            &buffer,
            cursor,
            temp.path().to_str().expect("utf8"),
        )
        .expect("complete")
        .expect("some result");

        let rendered = result.rendered_candidates.expect("candidate list");
        assert!(rendered.contains("alpha.txt"));
        assert!(rendered.contains("alpine.txt"));
        assert_eq!(editor.buffer(), "@alp");
    }

    #[test]
    fn thread_list_is_numbered_and_extractable() {
        let result = json!({
            "data": [
                {
                    "id": "thr_1",
                    "preview": "first thread",
                    "status": {"type": "idle"},
                    "updatedAt": 1
                },
                {
                    "id": "thr_2",
                    "preview": "second thread",
                    "status": {"type": "active"},
                    "updatedAt": 2
                }
            ]
        });
        let rendered = render_thread_list(&result, None);
        assert!(rendered.contains(" 1. thr_1"));
        assert!(rendered.contains("Use /resume <n>"));
        assert_eq!(extract_thread_ids(&result), vec!["thr_1", "thr_2"]);
    }

    #[test]
    fn resume_helpers_only_keep_recent_conversation_context() {
        let turns = vec![
            json!({
                "items": [
                    {"type": "userMessage", "content": [{"type": "text", "text": "old objective"}]},
                    {"type": "agentMessage", "text": "old reply"},
                    {"type": "reasoning", "text": "ignore"}
                ]
            }),
            json!({
                "items": [
                    {"type": "userMessage", "content": [{"type": "text", "text": "latest request"}]},
                    {"type": "agentMessage", "text": "latest reply"}
                ]
            }),
        ];

        let mut state = AppState::new(true, false);
        seed_resumed_state_from_turns(&turns, &mut state);
        assert_eq!(state.objective.as_deref(), Some("latest request"));
        assert_eq!(state.last_agent_message.as_deref(), Some("latest reply"));

        let recent_items = latest_conversation_history_items(&turns, 2);
        assert_eq!(recent_items.len(), 2);
        assert_eq!(
            recent_items[0].get("type").and_then(Value::as_str),
            Some("userMessage")
        );
        assert_eq!(
            recent_items[1].get("type").and_then(Value::as_str),
            Some("agentMessage")
        );
    }

    #[test]
    fn file_search_paths_are_extractable_for_numeric_insert() {
        let files = vec![
            json!({"path": "src/main.rs", "score": 1}),
            json!({"path": "src/lib.rs", "score": 2}),
        ];
        assert_eq!(
            extract_file_search_paths(&files),
            vec!["src/main.rs", "src/lib.rs"]
        );
        assert_eq!(quote_if_needed("src/main.rs"), "src/main.rs");
        assert_eq!(
            quote_if_needed("path with spaces.rs"),
            "\"path with spaces.rs\""
        );
    }

    #[test]
    fn fuzzy_file_search_rendering_shows_ranked_paths() {
        let rendered = render_fuzzy_file_search_results(
            "agent",
            &[
                json!({"path": "src/agent.rs", "score": 99}),
                json!({"path": "tests/agent_test.rs", "score": 78}),
            ],
        );
        assert!(rendered.contains("Query: agent"));
        assert!(rendered.contains("1. src/agent.rs  [score 99]"));
        assert!(rendered.contains("2. tests/agent_test.rs  [score 78]"));
    }

    #[test]
    fn slash_completion_rendering_includes_descriptions() {
        let rendered = render_slash_completion_candidates("re", &["resume", "review"], false);
        assert!(rendered.contains("/resume"));
        assert!(rendered.contains("resume a saved thread"));
        assert!(rendered.contains("/review"));
    }

    #[test]
    fn bare_slash_completion_uses_native_like_order() {
        let rendered = render_slash_completion_candidates("", builtin_command_names(), false);
        let model_pos = rendered.find("/model").expect("model should be listed");
        let review_pos = rendered.find("/review").expect("review should be listed");
        let new_pos = rendered.find("/new").expect("new should be listed");
        assert!(model_pos < review_pos);
        assert!(review_pos < new_pos);
    }

    #[test]
    fn help_lines_are_derived_from_command_metadata() {
        let rendered = builtin_help_lines().join("\n");
        assert!(rendered.contains(":resume [thread-id|n]"));
        assert!(rendered.contains("resume a saved thread"));
        assert!(rendered.contains(":plan"));
        assert!(rendered.contains("toggle plan collaboration mode"));
        assert!(rendered.contains(":approvals or /permissions"));
        assert!(rendered.contains(":ps [clean]"));
        assert!(rendered.contains("stop all background terminals"));
        assert!(rendered.contains(":realtime [start [prompt...]|send <text>|stop|status]"));
    }

    #[test]
    fn app_list_rendering_includes_slug_and_status() {
        let rendered = render_apps_list(&[AppCatalogEntry {
            id: "connector-1".to_string(),
            name: "Demo App".to_string(),
            slug: "demo-app".to_string(),
            enabled: true,
        }]);
        assert!(rendered.contains("Demo App"));
        assert!(rendered.contains("$demo-app"));
        assert!(rendered.contains("[enabled]"));
    }

    #[test]
    fn rate_limit_lines_show_remaining_capacity_and_reset() {
        let lines = render_rate_limit_lines(Some(&json!({
            "primary": {
                "usedPercent": 25,
                "windowDurationMins": 300,
                "resetsAt": 2200000000i64
            },
            "secondary": null
        })));
        assert!(lines[0].contains("5h limit 75% left"));
        assert!(lines[0].contains("resets"));
    }

    #[test]
    fn collaboration_modes_are_extractable_from_response() {
        let presets = extract_collaboration_mode_presets(&json!({
            "data": [
                {
                    "name": "Plan",
                    "mode": "plan",
                    "model": "gpt-5-codex",
                    "reasoning_effort": "high"
                },
                {
                    "name": "Default",
                    "mode": "default",
                    "model": "gpt-5-codex",
                    "reasoning_effort": null
                }
            ]
        }));
        assert_eq!(presets.len(), 2);
        assert!(presets[0].is_plan());
        assert_eq!(presets[1].mode_kind.as_deref(), Some("default"));
    }

    #[test]
    fn collaboration_mode_rendering_shows_current_and_available_presets() {
        let mut state = AppState::new(true, false);
        let presets = extract_collaboration_mode_presets(&json!({
            "data": [
                {
                    "name": "Plan",
                    "mode": "plan",
                    "model": "gpt-5-codex",
                    "reasoning_effort": "high"
                }
            ]
        }));
        state.collaboration_modes = presets.clone();
        state.active_collaboration_mode = Some(presets[0].clone());
        let rendered = render_collaboration_modes(&state);
        assert!(rendered.contains("current         Plan"));
        assert!(rendered.contains("mode=plan"));
        assert!(rendered.contains("Use /collab <name|mode> or /plan to switch."));
    }

    #[test]
    fn experimental_feature_rendering_shows_stage_status_and_key() {
        let rendered = render_experimental_features_list(&json!({
            "data": [
                {
                    "name": "realtime_conversation",
                    "stage": "beta",
                    "displayName": "Realtime conversation",
                    "description": "Enable the experimental realtime voice workflow.",
                    "announcement": "Try voice mode in supported clients.",
                    "enabled": true,
                    "defaultEnabled": false
                }
            ],
            "nextCursor": null
        }));
        assert!(rendered.contains("Realtime conversation  [beta] [enabled]"));
        assert!(rendered.contains("key: realtime_conversation"));
        assert!(rendered.contains("Enable the experimental realtime voice workflow."));
    }

    #[test]
    fn models_are_extractable_with_personality_support() {
        let models = extract_models(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                },
                {
                    "id": "legacy-model",
                    "displayName": "Legacy",
                    "supportsPersonality": false,
                    "isDefault": false
                }
            ]
        }));
        assert_eq!(models.len(), 2);
        assert!(models[0].supports_personality);
        assert!(models[0].is_default);
        assert!(!models[1].supports_personality);
    }

    #[test]
    fn personality_rendering_shows_current_and_model_support() {
        let mut state = AppState::new(true, false);
        state.models = extract_models(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                }
            ]
        }));
        state.active_personality = Some("pragmatic".to_string());
        let cli = Cli {
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
        };
        let rendered = render_personality_options(&cli, &state);
        assert_eq!(summarize_active_personality(&state), "Pragmatic");
        assert!(rendered.contains("current          Pragmatic"));
        assert!(rendered.contains("current model     GPT-5 Codex [supports personality]"));
    }

    #[test]
    fn models_render_default_and_personality_support_markers() {
        let rendered = render_models_list(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                },
                {
                    "id": "legacy-model",
                    "displayName": "Legacy",
                    "supportsPersonality": false,
                    "isDefault": false
                }
            ]
        }));
        assert!(rendered.contains("GPT-5 Codex (gpt-5-codex) [default] [supports personality]"));
        assert!(rendered.contains("Legacy (legacy-model) [personality unsupported]"));
    }

    #[test]
    fn status_snapshot_surfaces_effective_model_personality_support() {
        let mut state = AppState::new(true, false);
        state.models = extract_models(&json!({
            "data": [
                {
                    "id": "gpt-5-codex",
                    "displayName": "GPT-5 Codex",
                    "supportsPersonality": true,
                    "isDefault": true
                }
            ]
        }));
        let cli = Cli {
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
        };
        let rendered = render_status_snapshot(&cli, "/tmp/project", &state);
        assert!(rendered.contains("model           GPT-5 Codex [supports personality]"));
        assert!(rendered.contains("models cached   1"));
    }

    #[test]
    fn prompt_visibility_and_input_follow_runtime_state() {
        let mut state = AppState::new(true, false);
        assert!(!prompt_is_visible(&state));
        assert!(!prompt_accepts_input(&state));

        state.thread_id = Some("thread-1".to_string());
        assert!(prompt_is_visible(&state));
        assert!(prompt_accepts_input(&state));

        state.pending_thread_switch = true;
        assert!(!prompt_is_visible(&state));
        assert!(!prompt_accepts_input(&state));

        state.pending_thread_switch = false;
        state.active_exec_process_id = Some("proc-1".to_string());
        assert!(prompt_is_visible(&state));
        assert!(!prompt_accepts_input(&state));
    }

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
        let mut state = AppState::new(true, false);
        state.turn_running = true;
        state.started_turn_count = 2;
        state.last_status_line = Some("waiting on approval".to_string());
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("waiting on approval"));
    }

    #[test]
    fn prompt_status_mentions_plan_mode_when_selected() {
        let mut state = AppState::new(true, false);
        state.active_collaboration_mode = Some(CollaborationModePreset {
            name: "Plan".to_string(),
            mode_kind: Some("plan".to_string()),
            model: Some("gpt-5-codex".to_string()),
            reasoning_effort: Some(Some("high".to_string())),
        });
        assert_eq!(
            summarize_active_collaboration_mode(&state),
            "Plan (mode=plan, model=gpt-5-codex, effort=high)"
        );
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("plan mode"));
    }

    #[test]
    fn prompt_status_mentions_personality_when_selected() {
        let mut state = AppState::new(true, false);
        state.active_personality = Some("friendly".to_string());
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("Friendly"));
    }

    #[test]
    fn prompt_status_mentions_realtime_when_active() {
        let mut state = AppState::new(true, false);
        state.realtime_active = true;
        let rendered = render_prompt_status(&state);
        assert!(rendered.contains("realtime"));
    }

    #[test]
    fn status_snapshot_includes_realtime_fields() {
        let mut state = AppState::new(true, false);
        state.thread_id = Some("thread-1".to_string());
        state.realtime_active = true;
        state.realtime_session_id = Some("rt-1".to_string());
        state.realtime_prompt = Some("hello world".to_string());
        state.realtime_last_error = Some("bad gateway".to_string());
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
            prompt: Vec::new(),
        });
        let rendered = render_status_snapshot(&cli, "/tmp/project", &state);
        assert!(rendered.contains("realtime        true"));
        assert!(rendered.contains("realtime id     rt-1"));
        assert!(rendered.contains("realtime prompt hello world"));
        assert!(rendered.contains("realtime error  bad gateway"));
    }

    #[test]
    fn realtime_item_prefers_text_content() {
        let rendered = render_realtime_item(&json!({
            "type": "message",
            "id": "msg-1",
            "role": "assistant",
            "content": [
                {"text": "first line"},
                {"transcript": "second line"}
            ]
        }));
        assert!(rendered.contains("type            message"));
        assert!(rendered.contains("id              msg-1"));
        assert!(rendered.contains("role            assistant"));
        assert!(rendered.contains("first line"));
        assert!(rendered.contains("second line"));
    }

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
}
