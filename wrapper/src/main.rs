mod editor;
mod input;
mod output;
mod prompt;
mod render;
mod rpc;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;
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
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use clap::ArgAction;
use clap::Parser;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::terminal;
use editor::EditorEvent;
use editor::LineEditor;
use input::AppCatalogEntry;
use input::ParsedInput;
use input::PluginCatalogEntry;
use input::SkillCatalogEntry;
use input::build_turn_input;
use output::Output;
use prompt::build_continue_prompt;
use prompt::parse_auto_mode_stop;
use rpc::IncomingMessage;
use rpc::OutgoingErrorObject;
use rpc::OutgoingErrorResponse;
use rpc::OutgoingNotification;
use rpc::OutgoingRequest;
use rpc::OutgoingResponse;
use rpc::RequestId;
use rpc::RpcNotification;
use rpc::RpcRequest;
use rpc::RpcResponse;
use serde_json::Value;
use serde_json::json;

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

#[derive(Debug)]
enum PendingRequest {
    Initialize,
    LoadApps,
    LoadSkills,
    LoadAccount,
    LogoutAccount,
    UploadFeedback { classification: String },
    LoadRateLimits,
    LoadModels,
    LoadConfig,
    LoadMcpServers,
    ListThreads { search_term: Option<String> },
    StartThread { initial_prompt: Option<String> },
    ResumeThread { initial_prompt: Option<String> },
    ForkThread { initial_prompt: Option<String> },
    CompactThread,
    RenameThread { name: String },
    CleanBackgroundTerminals,
    StartReview { target_description: String },
    StartTurn { auto_generated: bool },
    SteerTurn { display_text: String },
    InterruptTurn,
    ExecCommand { process_id: String, command: String },
    TerminateExecCommand { process_id: String },
    FuzzyFileSearch { query: String },
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
    apps: Vec<AppCatalogEntry>,
    plugins: Vec<PluginCatalogEntry>,
    skills: Vec<SkillCatalogEntry>,
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
            apps: Vec::new(),
            plugins: Vec::new(),
            skills: Vec::new(),
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
        self.turn_running = false;
        self.activity_started_at = None;
        self.started_turn_count = 0;
        self.completed_turn_count = 0;
        self.objective = None;
        self.last_agent_message = None;
        self.last_turn_diff = None;
        self.last_token_usage = None;
        self.last_status_line = None;
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

fn send_initialize(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    experimental_api: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::Initialize);
    let mut capabilities = json!({
        "experimentalApi": experimental_api,
    });
    if !cli.raw_json {
        capabilities["optOutNotificationMethods"] = json!([
            "item/agentMessage/delta",
            "item/reasoning/summaryTextDelta",
            "item/reasoning/summaryPartAdded",
            "item/reasoning/textDelta",
            "item/plan/delta"
        ]);
    }
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "initialize",
            params: json!({
                "clientInfo": {
                    "name": "codexw_terminal",
                    "title": "CodexW Terminal",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": capabilities
            }),
        },
    )
}

fn send_initialized(writer: &mut ChildStdin) -> Result<()> {
    send_json(
        writer,
        &OutgoingNotification {
            method: "initialized",
            params: Value::Null,
        },
    )
}

fn send_load_apps(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadApps);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "app/list",
            params: json!({}),
        },
    )
}

fn send_load_skills(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadSkills);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "skills/list",
            params: json!({
                "cwds": [resolved_cwd],
            }),
        },
    )
}

fn send_load_account(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadAccount);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "account/read",
            params: json!({
                "refreshToken": false,
            }),
        },
    )
}

fn send_logout_account(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LogoutAccount);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "account/logout",
            params: json!({}),
        },
    )
}

fn send_feedback_upload(
    writer: &mut ChildStdin,
    state: &mut AppState,
    classification: String,
    reason: Option<String>,
    thread_id: Option<String>,
    include_logs: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::UploadFeedback {
            classification: classification.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "feedback/upload",
            params: json!({
                "classification": classification,
                "reason": reason,
                "threadId": thread_id,
                "includeLogs": include_logs,
            }),
        },
    )
}

fn send_load_rate_limits(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadRateLimits);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "account/rateLimits/read",
            params: json!({}),
        },
    )
}

fn send_load_models(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadModels);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "model/list",
            params: json!({
                "includeHidden": false,
            }),
        },
    )
}

fn send_load_config(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadConfig);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "config/read",
            params: json!({}),
        },
    )
}

fn send_load_mcp_servers(writer: &mut ChildStdin, state: &mut AppState) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::LoadMcpServers);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "mcpServerStatus/list",
            params: json!({
                "limit": 50,
            }),
        },
    )
}

fn send_list_threads(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
    search_term: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ListThreads {
            search_term: search_term.clone(),
        },
    );
    let mut params = json!({
        "limit": 10,
        "sortKey": "updated_at",
        "cwd": resolved_cwd,
    });
    if let Some(search_term) = search_term {
        params["searchTerm"] = Value::String(search_term);
    }
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/list",
            params,
        },
    )
}

fn send_fuzzy_file_search(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
    query: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::FuzzyFileSearch {
            query: query.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "fuzzyFileSearch",
            params: json!({
                "query": query,
                "roots": [resolved_cwd],
            }),
        },
    )
}

fn send_thread_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    initial_prompt: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending_thread_switch = true;
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartThread { initial_prompt },
    );

    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/start",
            params: json!({
                "model": cli.model,
                "modelProvider": cli.model_provider,
                "cwd": resolved_cwd,
                "approvalPolicy": approval_policy(cli),
                "sandbox": thread_sandbox_mode(cli),
                "serviceName": "codexw_terminal",
                "experimentalRawEvents": false,
            }),
        },
    )
}

fn send_thread_resume(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: String,
    initial_prompt: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending_thread_switch = true;
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ResumeThread { initial_prompt },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/resume",
            params: json!({
                "threadId": thread_id,
                "model": cli.model,
                "modelProvider": cli.model_provider,
                "cwd": resolved_cwd,
                "approvalPolicy": approval_policy(cli),
                "sandbox": thread_sandbox_mode(cli),
            }),
        },
    )
}

fn send_thread_fork(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: String,
    initial_prompt: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending_thread_switch = true;
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ForkThread { initial_prompt },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/fork",
            params: json!({
                "threadId": thread_id,
                "cwd": resolved_cwd,
                "model": cli.model,
                "modelProvider": cli.model_provider,
                "approvalPolicy": approval_policy(cli),
                "sandbox": thread_sandbox_mode(cli),
            }),
        },
    )
}

fn send_thread_compact(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::CompactThread);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/compact/start",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}

fn send_thread_rename(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    name: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::RenameThread { name: name.clone() },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/name/set",
            params: json!({
                "threadId": thread_id,
                "name": name,
            }),
        },
    )
}

fn send_clean_background_terminals(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::CleanBackgroundTerminals);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/backgroundTerminals/clean",
            params: json!({
                "threadId": thread_id,
            }),
        },
    )
}

fn send_start_review(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    review_target: Value,
    target_description: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartReview { target_description },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "review/start",
            params: json!({
                "threadId": thread_id,
                "delivery": "inline",
                "target": review_target,
            }),
        },
    )
}

fn send_turn_start(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    thread_id: String,
    submission: ParsedInput,
    auto_generated: bool,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::StartTurn { auto_generated },
    );
    if !auto_generated && state.objective.is_none() && !submission.display_text.trim().is_empty() {
        state.objective = Some(submission.display_text.clone());
    }

    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/start",
            params: json!({
                "threadId": thread_id,
                "input": submission.items,
                "cwd": resolved_cwd,
                "approvalPolicy": approval_policy(cli),
                "sandboxPolicy": turn_sandbox_policy(cli),
                "model": cli.model,
                "summary": reasoning_summary(cli),
            }),
        },
    )
}

fn send_turn_steer(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    turn_id: String,
    submission: ParsedInput,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::SteerTurn {
            display_text: submission.display_text.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/steer",
            params: json!({
                "threadId": thread_id,
                "expectedTurnId": turn_id,
                "input": submission.items,
            }),
        },
    )
}

fn send_turn_interrupt(
    writer: &mut ChildStdin,
    state: &mut AppState,
    thread_id: String,
    turn_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state
        .pending
        .insert(request_id.clone(), PendingRequest::InterruptTurn);
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "turn/interrupt",
            params: json!({
                "threadId": thread_id,
                "turnId": turn_id,
            }),
        },
    )
}

fn send_command_exec(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cli: &Cli,
    resolved_cwd: &str,
    command: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    let process_id = format!("codexw-cmd-{}", state.next_request_id);
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ExecCommand {
            process_id: process_id.clone(),
            command: command.clone(),
        },
    );
    state.process_output_buffers.remove(&process_id);
    state.active_exec_process_id = Some(process_id.clone());
    state.activity_started_at = Some(Instant::now());
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "command/exec",
            params: json!({
                "command": [shell_program(), "-lc", command],
                "processId": process_id,
                "cwd": resolved_cwd,
                "streamStdoutStderr": true,
                "sandboxPolicy": turn_sandbox_policy(cli),
            }),
        },
    )
}

fn send_command_exec_terminate(
    writer: &mut ChildStdin,
    state: &mut AppState,
    process_id: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::TerminateExecCommand {
            process_id: process_id.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "command/exec/terminate",
            params: json!({
                "processId": process_id,
            }),
        },
    )
}

fn process_server_line(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if state.raw_json {
        output.line_stderr(format!("[json] {line}"))?;
    }
    match rpc::parse_line(&line) {
        Ok(IncomingMessage::Response(response)) => {
            handle_response(
                response,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
                start_after_initialize,
            )?;
        }
        Ok(IncomingMessage::Request(request)) => {
            handle_server_request(request, cli, output, writer)?;
        }
        Ok(IncomingMessage::Notification(notification)) => {
            handle_notification(notification, cli, resolved_cwd, state, output, writer)?;
        }
        Err(err) => {
            output.line_stderr(format!("[session] ignored malformed server line: {err}"))?;
        }
    }
    Ok(())
}

fn handle_response(
    response: RpcResponse,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    let pending = state.pending.remove(&response.id);
    if let Some(error) = response.error {
        return handle_response_error(error, pending, state, output);
    }

    let Some(pending) = pending else {
        return Ok(());
    };

    match pending {
        PendingRequest::Initialize => {
            send_initialized(writer)?;
            output.line_stderr("[session] connected")?;
            if let Some(start_mode) = start_after_initialize.take() {
                match start_mode.resume_thread_id {
                    Some(thread_id) => {
                        output.line_stderr(format!("[thread] resume {thread_id}"))?;
                        send_thread_resume(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            thread_id,
                            start_mode.initial_prompt,
                        )?
                    }
                    None => {
                        output.line_stderr("[thread] create")?;
                        send_thread_start(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            start_mode.initial_prompt,
                        )?
                    }
                }
            }
            send_load_apps(writer, state)?;
            send_load_skills(writer, state, resolved_cwd)?;
            send_load_account(writer, state)?;
            send_load_rate_limits(writer, state)?;
        }
        PendingRequest::StartThread { initial_prompt } => {
            state.pending_thread_switch = false;
            state.reset_thread_context();
            let thread_id = get_string(&response.result, &["thread", "id"])
                .context("thread/start missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] started {thread_id}"))?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    &text,
                    resolved_cwd,
                    &[],
                    &[],
                    &state.apps,
                    &state.plugins,
                    &state.skills,
                );
                send_turn_start(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id,
                    submission,
                    false,
                )?;
            }
        }
        PendingRequest::ResumeThread { initial_prompt } => {
            state.pending_thread_switch = false;
            state.reset_thread_context();
            let thread_id = get_string(&response.result, &["thread", "id"])
                .context("thread/resume missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] resumed {thread_id}"))?;
            render_resumed_history(&response.result, state, output)?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    &text,
                    resolved_cwd,
                    &[],
                    &[],
                    &state.apps,
                    &state.plugins,
                    &state.skills,
                );
                send_turn_start(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id,
                    submission,
                    false,
                )?;
            }
        }
        PendingRequest::ForkThread { initial_prompt } => {
            state.pending_thread_switch = false;
            state.reset_thread_context();
            let thread_id = get_string(&response.result, &["thread", "id"])
                .context("thread/fork missing thread.id")?
                .to_string();
            state.thread_id = Some(thread_id.clone());
            output.line_stderr(format!("[thread] forked to {thread_id}"))?;
            render_resumed_history(&response.result, state, output)?;
            if let Some(text) = initial_prompt {
                let submission = build_turn_input(
                    &text,
                    resolved_cwd,
                    &[],
                    &[],
                    &state.apps,
                    &state.plugins,
                    &state.skills,
                );
                send_turn_start(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id,
                    submission,
                    false,
                )?;
            }
        }
        PendingRequest::CompactThread => {
            output.line_stderr("[thread] compaction requested")?;
        }
        PendingRequest::RenameThread { name } => {
            output.line_stderr(format!("[thread] renamed to {}", summarize_text(&name)))?;
        }
        PendingRequest::CleanBackgroundTerminals => {
            output.line_stderr("[thread] background terminal cleanup requested")?;
        }
        PendingRequest::StartReview { target_description } => {
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.reset_turn_stream_state();
            output.line_stderr(format!(
                "[review] started {}",
                summarize_text(&target_description)
            ))?;
        }
        PendingRequest::StartTurn { auto_generated } => {
            let turn_id = get_string(&response.result, &["turn", "id"])
                .context("turn/start missing turn.id")?
                .to_string();
            state.active_turn_id = Some(turn_id.clone());
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.reset_turn_stream_state();
            if auto_generated {
                output.line_stderr("[auto] starting follow-up turn")?;
            }
        }
        PendingRequest::SteerTurn { display_text } => {
            let turn_id = get_string(&response.result, &["turnId"])
                .context("turn/steer missing turnId")?
                .to_string();
            state.active_turn_id = Some(turn_id);
            output.line_stderr(format!("[steer] {}", summarize_text(&display_text)))?;
        }
        PendingRequest::InterruptTurn => {
            output.line_stderr("[interrupt] requested")?;
        }
        PendingRequest::LoadApps => {
            state.apps = parse_apps_list(&response.result);
        }
        PendingRequest::LoadSkills => {
            state.skills = parse_skills_list(&response.result, resolved_cwd);
        }
        PendingRequest::LoadAccount => {
            state.account_info = response.result.get("account").cloned();
        }
        PendingRequest::LogoutAccount => {
            state.account_info = None;
            state.rate_limits = None;
            output.line_stderr("[session] logged out")?;
            send_load_account(writer, state)?;
            send_load_rate_limits(writer, state)?;
        }
        PendingRequest::UploadFeedback { classification } => {
            let tracking_thread = get_string(&response.result, &["threadId"]).unwrap_or("-");
            output.line_stderr(format!(
                "[feedback] submitted {} feedback; tracking thread {}",
                summarize_text(&classification),
                tracking_thread
            ))?;
        }
        PendingRequest::LoadRateLimits => {
            state.rate_limits = response.result.get("rateLimits").cloned();
        }
        PendingRequest::LoadModels => {
            output.block_stdout("Models", &render_models_list(&response.result))?;
        }
        PendingRequest::LoadConfig => {
            output.block_stdout("Config", &render_config_snapshot(&response.result))?;
        }
        PendingRequest::LoadMcpServers => {
            output.block_stdout("MCP servers", &render_mcp_server_list(&response.result))?;
        }
        PendingRequest::ListThreads { search_term } => {
            state.last_listed_thread_ids = extract_thread_ids(&response.result);
            output.block_stdout(
                "Threads",
                &render_thread_list(&response.result, search_term.as_deref()),
            )?;
        }
        PendingRequest::ExecCommand {
            process_id,
            command,
        } => {
            let exit_code = response
                .result
                .get("exitCode")
                .and_then(Value::as_i64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string());
            let buffer = state
                .process_output_buffers
                .remove(&process_id)
                .unwrap_or_default();
            let stdout = if buffer.stdout.trim().is_empty() {
                get_string(&response.result, &["stdout"])
                    .unwrap_or("")
                    .to_string()
            } else {
                buffer.stdout
            };
            let stderr = if buffer.stderr.trim().is_empty() {
                get_string(&response.result, &["stderr"])
                    .unwrap_or("")
                    .to_string()
            } else {
                buffer.stderr
            };
            state.active_exec_process_id = None;
            state.activity_started_at = None;
            state.last_status_line = None;
            output.block_stdout(
                "Local command",
                &render_local_command_completion(&command, &exit_code, &stdout, &stderr),
            )?;
        }
        PendingRequest::TerminateExecCommand { process_id } => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.activity_started_at = None;
                output.line_stderr("[interrupt] local command termination requested")?;
            }
        }
        PendingRequest::FuzzyFileSearch { query } => {
            let files = response
                .result
                .get("files")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            state.last_file_search_paths = extract_file_search_paths(&files);
            let rendered = render_fuzzy_file_search_results(&query, files.as_slice());
            output.block_stdout("File mentions", &rendered)?;
        }
    }

    Ok(())
}

fn handle_response_error(
    error: Value,
    pending: Option<PendingRequest>,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    match pending {
        Some(PendingRequest::LoadRateLimits) => {
            output.line_stderr("[session] rate limits unavailable for the current account")?;
        }
        Some(PendingRequest::LoadAccount) => {
            output.line_stderr("[session] account details unavailable from app-server")?;
        }
        Some(PendingRequest::LogoutAccount) => {
            output.line_stderr("[session] logout failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::UploadFeedback { classification }) => {
            output.line_stderr(format!(
                "[feedback] failed to submit {} feedback",
                summarize_text(&classification)
            ))?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::StartThread { .. })
        | Some(PendingRequest::ResumeThread { .. })
        | Some(PendingRequest::ForkThread { .. }) => {
            state.pending_thread_switch = false;
            output.line_stderr("[thread] failed to switch threads")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::ExecCommand { process_id, .. }) => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            state.process_output_buffers.remove(&process_id);
            state.last_status_line = None;
            output.line_stderr("[command] failed to start local command")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::TerminateExecCommand { process_id }) => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            output.line_stderr("[command] failed to terminate local command cleanly")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        _ => {
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
    }
    Ok(())
}

fn handle_server_request(
    request: RpcRequest,
    cli: &Cli,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    match request.method.as_str() {
        "item/commandExecution/requestApproval" => {
            let decision_value = choose_command_approval_decision(&request.params, cli.yolo);
            output.line_stderr(format!(
                "[approval] {}",
                summarize_command_approval_request(&request.params, &decision_value)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({"decision": decision_value}),
                },
            )?;
        }
        "item/fileChange/requestApproval" | "execCommandApproval" | "applyPatchApproval" => {
            let decision = params_auto_approval_result(&request.params);
            output.line_stderr(format!(
                "[approval] {}",
                summarize_generic_approval_request(&request.params, &request.method)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: decision,
                },
            )?;
        }
        "tool/requestUserInput" | "item/tool/requestUserInput" => {
            let result = build_tool_user_input_response(&request.params);
            output.line_stderr(format!(
                "[input-request] auto-answered: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result,
                },
            )?;
        }
        "mcpServer/elicitation/request" => {
            output.line_stderr(format!(
                "[input-request] auto-cancelled MCP elicitation: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({"action": "cancel", "content": Value::Null}),
                },
            )?;
        }
        "item/tool/call" => {
            output.line_stderr(format!(
                "[tool] unsupported dynamic tool call: {}",
                summarize_tool_request(&request.params)
            ))?;
            send_json(
                writer,
                &OutgoingResponse {
                    id: request.id,
                    result: json!({
                        "contentItems": [
                            {
                                "type": "inputText",
                                "text": "codexw does not implement dynamic tool calls"
                            }
                        ],
                        "success": false
                    }),
                },
            )?;
        }
        _ => {
            if cli.verbose_events || cli.raw_json {
                output.line_stderr(format!(
                    "[server-request] {}: {}",
                    request.method,
                    if cli.raw_json {
                        serde_json::to_string_pretty(&request.params)?
                    } else {
                        summarize_value(&request.params)
                    }
                ))?;
            }
            send_json(
                writer,
                &OutgoingErrorResponse {
                    id: request.id,
                    error: OutgoingErrorObject {
                        code: -32601,
                        message: format!("codexw does not implement {}", request.method),
                        data: None,
                    },
                },
            )?;
        }
    }
    Ok(())
}

fn params_auto_approval_result(params: &Value) -> Value {
    if let Some(decisions) = params.get("availableDecisions").and_then(Value::as_array)
        && let Some(decision) = choose_first_allowed_decision(decisions)
    {
        return json!({"decision": decision});
    }
    json!({"decision": "accept"})
}

fn handle_notification(
    notification: RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    match notification.method.as_str() {
        "thread/started" => {
            if let Some(thread_id) = get_string(&notification.params, &["thread", "id"]) {
                state.thread_id = Some(thread_id.to_string());
            }
        }
        "skills/changed" => {
            send_load_skills(writer, state, resolved_cwd)?;
        }
        "app/list/updated" => {
            state.apps = parse_apps_list(&notification.params);
        }
        "account/updated" => {
            state.account_info = notification.params.get("account").cloned().or_else(|| {
                let auth_mode = notification.params.get("authMode")?.clone();
                let plan_type = notification
                    .params
                    .get("planType")
                    .cloned()
                    .unwrap_or(Value::Null);
                Some(json!({
                    "type": auth_mode,
                    "planType": plan_type,
                }))
            });
        }
        "account/rateLimits/updated" => {
            state.rate_limits = notification.params.get("rateLimits").cloned();
        }
        "thread/status/changed" => {
            if let Some(status_line) = summarize_thread_status_for_display(&notification.params) {
                emit_status_line(output, state, status_line)?;
            }
        }
        "thread/tokenUsage/updated" => {
            state.last_token_usage = notification.params.get("tokenUsage").cloned();
        }
        "turn/started" => {
            state.turn_running = true;
            state.activity_started_at = Some(Instant::now());
            state.started_turn_count = state.started_turn_count.saturating_add(1);
            if let Some(turn_id) = get_string(&notification.params, &["turn", "id"]) {
                state.active_turn_id = Some(turn_id.to_string());
            }
            state.reset_turn_stream_state();
            state.last_status_line = None;
        }
        "turn/completed" => {
            output.finish_stream()?;
            let status = get_string(&notification.params, &["turn", "status"])
                .unwrap_or("unknown")
                .to_string();
            let turn_id = get_string(&notification.params, &["turn", "id"])
                .unwrap_or("?")
                .to_string();
            state.turn_running = false;
            state.active_turn_id = None;
            state.activity_started_at = None;
            state.last_status_line = None;
            if matches!(
                status.as_str(),
                "completed" | "interrupted" | "failed" | "cancelled"
            ) {
                state.completed_turn_count = state.completed_turn_count.saturating_add(1);
            }
            if status != "completed" {
                output.line_stderr(format!("[turn] completed {turn_id} status={status}"))?;
            }

            if status == "completed" {
                if let Some(message) = state.last_agent_message.clone() {
                    let stop = parse_auto_mode_stop(&message);
                    if state.auto_continue && !stop {
                        let thread_id = thread_id(state)?.to_string();
                        let continue_prompt =
                            build_continue_prompt(state.objective.as_deref(), &message);
                        let submission = build_turn_input(
                            &continue_prompt,
                            resolved_cwd,
                            &[],
                            &[],
                            &state.apps,
                            &state.plugins,
                            &state.skills,
                        );
                        output.line_stderr("[auto] continuing remaining work")?;
                        send_turn_start(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            thread_id,
                            submission,
                            true,
                        )?;
                    } else if stop {
                        output.line_stderr("[ready] stop marker observed")?;
                    } else {
                        output.line_stderr("[ready]")?;
                    }
                } else {
                    output.line_stderr("[ready]")?;
                }
            } else {
                state.last_agent_message = None;
                output.line_stderr("[ready]")?;
            }
        }
        "command/exec/outputDelta" => {
            buffer_process_delta(&mut state.process_output_buffers, &notification.params);
        }
        "turn/diff/updated" => {
            state.last_turn_diff =
                get_string(&notification.params, &["diff"]).map(ToOwned::to_owned);
            if cli.verbose_events
                && let Some(diff) = get_string(&notification.params, &["diff"])
            {
                output.line_stdout("[diff]")?;
                output.line_stdout(diff)?;
            }
        }
        "turn/plan/updated" => {
            let plan_text = format_plan(&notification.params);
            if !plan_text.is_empty() {
                output.line_stdout("[plan]")?;
                output.line_stdout(plan_text)?;
            }
        }
        "model/rerouted" => {
            output.line_stderr(format!(
                "[model] {}",
                summarize_model_reroute(&notification.params)
            ))?;
        }
        "item/started" => render_item_started(&notification.params, state)?,
        "item/completed" => render_item_completed(&notification.params, state, output)?,
        "item/agentMessage/delta"
        | "item/reasoning/summaryTextDelta"
        | "item/reasoning/textDelta"
        | "item/reasoning/summaryPartAdded" => {}
        "item/commandExecution/outputDelta" => {
            buffer_item_delta(&mut state.command_output_buffers, &notification.params)
        }
        "item/fileChange/outputDelta" => {
            buffer_item_delta(&mut state.file_output_buffers, &notification.params)
        }
        "item/commandExecution/terminalInteraction" => {
            if cli.verbose_events
                && let Some(summary) = summarize_terminal_interaction(&notification.params)
            {
                output.line_stderr(format!("[command-input] {summary}"))?;
            }
        }
        "serverRequest/resolved" => {
            if cli.verbose_events {
                output.line_stderr(format!(
                    "[approval] resolved {}",
                    summarize_server_request_resolved(&notification.params)
                ))?;
            }
        }
        "error" => {
            output.line_stderr(format!(
                "[turn-error] {}",
                summarize_value(&notification.params)
            ))?;
        }
        other if other.starts_with("codex/event/") => {
            if other == "codex/event/task_complete" {
                if let Some(message) =
                    get_string(&notification.params, &["msg", "last_agent_message"])
                {
                    state.last_agent_message = Some(message.to_string());
                }
            } else if cli.verbose_events {
                output.line_stderr(format!(
                    "[event] {other}: {}",
                    if cli.raw_json {
                        serde_json::to_string_pretty(&notification.params)?
                    } else {
                        summarize_value(&notification.params)
                    }
                ))?;
            }
        }
        other => {
            if cli.verbose_events {
                output.line_stderr(format!(
                    "[event] {other}: {}",
                    if cli.raw_json {
                        serde_json::to_string_pretty(&notification.params)?
                    } else {
                        summarize_value(&notification.params)
                    }
                ))?;
            }
        }
    }
    Ok(())
}

fn render_item_started(params: &Value, state: &mut AppState) -> Result<()> {
    let Some(item) = params.get("item") else {
        return Ok(());
    };
    let item_type = get_string(item, &["type"]).unwrap_or("unknown");
    match item_type {
        "commandExecution" => {
            let command = get_string(item, &["command"]).unwrap_or("");
            state.last_status_line = Some(format!("running {}", summarize_text(command)));
        }
        "fileChange" => {
            state.last_status_line = Some(summarize_file_change_paths(item));
        }
        "agentMessage" | "reasoning" => {}
        "mcpToolCall" | "dynamicToolCall" | "collabAgentToolCall" | "webSearch" | "plan" => {
            state.last_status_line = Some(format!(
                "{}",
                summarize_text(&format!(
                    "{} {}",
                    humanize_item_type(item_type),
                    summarize_tool_item(item_type, item)
                ))
            ));
        }
        _ => {}
    }
    Ok(())
}

fn render_item_completed(params: &Value, state: &mut AppState, output: &mut Output) -> Result<()> {
    let Some(item) = params.get("item") else {
        return Ok(());
    };
    let item_type = get_string(item, &["type"]).unwrap_or("unknown");
    match item_type {
        "agentMessage" => {
            let text = get_string(item, &["text"]).unwrap_or("").to_string();
            state.last_agent_message = Some(text.clone());
            output.finish_stream()?;
            if !text.trim().is_empty() {
                output.block_stdout("Assistant", &text)?;
            }
        }
        "commandExecution" => {
            let status = get_string(item, &["status"]).unwrap_or("unknown");
            let command = get_string(item, &["command"]).unwrap_or("");
            let exit_code = item
                .get("exitCode")
                .and_then(Value::as_i64)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string());
            output.finish_stream()?;
            let item_id = get_string(item, &["id"]).unwrap_or("");
            let full_output = state
                .command_output_buffers
                .remove(item_id)
                .filter(|text| !text.trim().is_empty())
                .or_else(|| {
                    get_string(item, &["aggregatedOutput"])
                        .map(ToOwned::to_owned)
                        .filter(|text| !text.trim().is_empty())
                });
            let rendered =
                render_command_completion(command, status, &exit_code, full_output.as_deref());
            output.block_stdout("Command complete", &rendered)?;
        }
        "fileChange" => {
            let status = get_string(item, &["status"]).unwrap_or("unknown");
            output.finish_stream()?;
            let item_id = get_string(item, &["id"]).unwrap_or("");
            let delta_output = state
                .file_output_buffers
                .remove(item_id)
                .filter(|text| !text.trim().is_empty());
            let rendered = render_file_change_completion(item, status, delta_output.as_deref());
            output.block_stdout("File changes complete", &rendered)?;
        }
        "reasoning" => {
            output.finish_stream()?;
            let rendered = render_reasoning_item(item);
            if !rendered.is_empty() {
                output.block_stdout("Thinking", &rendered)?;
            }
        }
        "mcpToolCall" | "dynamicToolCall" | "collabAgentToolCall" | "webSearch" | "plan" => {
            output.finish_stream()?;
            output.block_stdout(
                &format!("{} complete", humanize_item_type(item_type)),
                &summarize_tool_item(item_type, item),
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_user_input(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(true);
    }

    if let Some(command) = trimmed.strip_prefix(':') {
        return handle_command(command, cli, resolved_cwd, state, editor, output, writer);
    }
    if let Some(command) = trimmed.strip_prefix('/') {
        if is_builtin_command(command) {
            return handle_command(command, cli, resolved_cwd, state, editor, output, writer);
        }
    }

    if let Some(command) = trimmed.strip_prefix('!') {
        if state.turn_running {
            output.line_stderr(
                "[session] wait for the active turn to finish before running a local command",
            )?;
            return Ok(true);
        }
        if state.active_exec_process_id.is_some() {
            output.line_stderr("[session] a local command is already running")?;
            return Ok(true);
        }
        let command = command.trim();
        if command.is_empty() {
            output.line_stderr("[session] usage: !<shell command>")?;
            return Ok(true);
        }
        emit_status_line(
            output,
            state,
            format!("running local command: {}", summarize_text(command)),
        )?;
        send_command_exec(writer, state, cli, resolved_cwd, command.to_string())?;
        return Ok(true);
    }

    let (local_images, remote_images) = state.take_pending_attachments();
    let submission = build_turn_input(
        trimmed,
        resolved_cwd,
        &local_images,
        &remote_images,
        &state.apps,
        &state.plugins,
        &state.skills,
    );
    if submission.items.is_empty() {
        output.line_stderr("[session] nothing to submit")?;
        return Ok(true);
    }

    let thread_id = thread_id(state)?.to_string();
    if state.turn_running {
        let turn_id = state
            .active_turn_id
            .clone()
            .context("turn is marked running but active turn id is missing")?;
        send_turn_steer(writer, state, thread_id, turn_id, submission)?;
    } else {
        send_turn_start(
            writer,
            state,
            cli,
            resolved_cwd,
            thread_id,
            submission,
            false,
        )?;
    }
    Ok(true)
}

fn handle_command(
    command_line: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let mut parts = command_line.split_whitespace();
    let Some(command) = parts.next() else {
        output.line_stderr("[session] empty command")?;
        return Ok(true);
    };

    match command {
        "help" | "h" => {
            for line in builtin_help_lines() {
                output.line_stderr(line)?;
            }
            output.line_stderr(
                "!<command>           run a local shell command via app-server command/exec",
            )?;
            Ok(true)
        }
        "quit" | "q" | "exit" => Ok(false),
        "new" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            output.line_stderr("[session] creating new thread")?;
            send_thread_start(writer, state, cli, resolved_cwd, None)?;
            Ok(true)
        }
        "resume" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let maybe_arg = parts.next().map(ToOwned::to_owned);
            let Some(thread_id) = maybe_arg else {
                output.line_stderr(
                    "[session] loading recent threads; use /resume <n> or /resume <thread-id>",
                )?;
                send_list_threads(writer, state, resolved_cwd, None)?;
                return Ok(true);
            };
            let thread_id = if let Ok(index) = thread_id.parse::<usize>() {
                match state.last_listed_thread_ids.get(index.saturating_sub(1)) {
                    Some(thread_id) => thread_id.clone(),
                    None => {
                        output.line_stderr("[session] no cached thread at that index; run /threads or /resume first")?;
                        return Ok(true);
                    }
                }
            } else {
                thread_id
            };
            output.line_stderr(format!("[session] resuming thread {thread_id}"))?;
            send_thread_resume(
                writer,
                state,
                cli,
                resolved_cwd,
                thread_id.to_string(),
                None,
            )?;
            Ok(true)
        }
        "fork" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            let initial_prompt = join_prompt(&parts.map(str::to_string).collect::<Vec<_>>());
            output.line_stderr(format!("[thread] forking {current_thread_id}"))?;
            send_thread_fork(
                writer,
                state,
                cli,
                resolved_cwd,
                current_thread_id,
                initial_prompt,
            )?;
            Ok(true)
        }
        "compact" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[thread] requesting compaction")?;
            send_thread_compact(writer, state, current_thread_id)?;
            Ok(true)
        }
        "review" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            let args = parts.collect::<Vec<_>>().join(" ");
            let trimmed_args = args.trim();
            let (target, description) = if trimmed_args.is_empty() {
                (
                    json!({"type": "uncommittedChanges"}),
                    "current uncommitted changes".to_string(),
                )
            } else {
                (
                    json!({"type": "custom", "instructions": trimmed_args}),
                    trimmed_args.to_string(),
                )
            };
            output.line_stderr(format!(
                "[review] requesting {}",
                summarize_text(&description)
            ))?;
            send_start_review(writer, state, current_thread_id, target, description)?;
            Ok(true)
        }
        "apps" => {
            output.block_stdout("Apps", &render_apps_list(&state.apps))?;
            Ok(true)
        }
        "skills" => {
            output.block_stdout("Skills", &render_skills_list(&state.skills))?;
            Ok(true)
        }
        "models" | "model" => {
            output.line_stderr("[session] loading models")?;
            send_load_models(writer, state)?;
            Ok(true)
        }
        "mcp" => {
            output.line_stderr("[session] loading MCP server status")?;
            send_load_mcp_servers(writer, state)?;
            Ok(true)
        }
        "clean" => {
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[thread] cleaning background terminals")?;
            send_clean_background_terminals(writer, state, current_thread_id)?;
            Ok(true)
        }
        "threads" => {
            let search_term = parts.collect::<Vec<_>>().join(" ");
            let search_term = search_term.trim();
            let search_term = if search_term.is_empty() {
                None
            } else {
                Some(search_term.to_string())
            };
            output.line_stderr("[session] loading recent threads")?;
            send_list_threads(writer, state, resolved_cwd, search_term)?;
            Ok(true)
        }
        "mention" => {
            let query = parts.collect::<Vec<_>>().join(" ");
            let query = query.trim();
            if query.is_empty() {
                editor.insert_str("@");
                return Ok(true);
            }
            if let Ok(index) = query.parse::<usize>() {
                let Some(path) = state
                    .last_file_search_paths
                    .get(index.saturating_sub(1))
                    .cloned()
                else {
                    output.line_stderr(
                        "[session] no cached file match at that index; run /mention <query> first",
                    )?;
                    return Ok(true);
                };
                let inserted = quote_if_needed(&path);
                editor.insert_str(&format!("{inserted} "));
                output.line_stderr(format!("[mention] inserted {}", summarize_text(&path)))?;
                return Ok(true);
            }
            output.line_stderr(format!("[search] files matching {}", summarize_text(query)))?;
            send_fuzzy_file_search(writer, state, resolved_cwd, query.to_string())?;
            Ok(true)
        }
        "diff" => {
            if let Some(diff) = state.last_turn_diff.as_deref() {
                output.block_stdout("Latest diff", diff)?;
            } else {
                output.line_stderr("[diff] no turn diff has been emitted yet")?;
            }
            Ok(true)
        }
        "clear" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                return Ok(true);
            }
            output.clear_screen()?;
            output.line_stderr("[thread] creating new thread after clear")?;
            send_thread_start(writer, state, cli, resolved_cwd, None)?;
            Ok(true)
        }
        "copy" => {
            if let Some(message) = state.last_agent_message.as_deref() {
                copy_to_clipboard(message, output)?;
            } else {
                output.line_stderr("[copy] no assistant reply is available yet")?;
            }
            Ok(true)
        }
        "auto" => {
            let Some(mode) = parts.next() else {
                output.line_stderr("[session] usage: :auto on|off")?;
                return Ok(true);
            };
            state.auto_continue = match mode {
                "on" => true,
                "off" => false,
                _ => {
                    output.line_stderr("[session] usage: :auto on|off")?;
                    return Ok(true);
                }
            };
            output.line_stderr(format!(
                "[auto] {}",
                if state.auto_continue {
                    "enabled"
                } else {
                    "disabled"
                }
            ))?;
            Ok(true)
        }
        "attach-image" | "attach" => {
            let Some(path) = parts.next() else {
                output.line_stderr("[session] usage: :attach-image <path>")?;
                return Ok(true);
            };
            let path = canonicalize_or_keep(path);
            state.pending_local_images.push(path.clone());
            output.line_stderr(format!("[draft] queued local image {path}"))?;
            Ok(true)
        }
        "attach-url" => {
            let Some(url) = parts.next() else {
                output.line_stderr("[session] usage: :attach-url <url>")?;
                return Ok(true);
            };
            state.pending_remote_images.push(url.to_string());
            output.line_stderr(format!("[draft] queued remote image {url}"))?;
            Ok(true)
        }
        "attachments" => {
            if state.pending_local_images.is_empty() && state.pending_remote_images.is_empty() {
                output.line_stderr("[draft] no queued attachments")?;
                return Ok(true);
            }
            let rendered = render_pending_attachments(
                &state.pending_local_images,
                &state.pending_remote_images,
            );
            output.block_stdout("Queued attachments", &rendered)?;
            Ok(true)
        }
        "clear-attachments" => {
            state.pending_local_images.clear();
            state.pending_remote_images.clear();
            output.line_stderr("[draft] cleared queued attachments")?;
            Ok(true)
        }
        "interrupt" => {
            if let Some(turn_id) = state.active_turn_id.clone() {
                output.line_stderr("[interrupt] interrupting active turn")?;
                send_turn_interrupt(writer, state, thread_id(state)?.to_string(), turn_id)?;
            } else if let Some(process_id) = state.active_exec_process_id.clone() {
                output.line_stderr("[interrupt] terminating active local command")?;
                send_command_exec_terminate(writer, state, process_id)?;
            } else {
                output.line_stderr("[interrupt] no active turn")?;
            }
            Ok(true)
        }
        "rename" => {
            let name = parts.collect::<Vec<_>>().join(" ").trim().to_string();
            if name.is_empty() {
                output.line_stderr("[session] usage: :rename <name>")?;
                return Ok(true);
            }
            let current_thread_id = thread_id(state)?.to_string();
            send_thread_rename(writer, state, current_thread_id, name)?;
            Ok(true)
        }
        "approvals" | "permissions" => {
            output.block_stdout("Permissions", &render_permissions_snapshot(cli))?;
            Ok(true)
        }
        "status" => {
            output.block_stdout("Status", &render_status_snapshot(cli, resolved_cwd, state))?;
            Ok(true)
        }
        "statusline" => {
            output.block_stdout("Status", &render_status_snapshot(cli, resolved_cwd, state))?;
            Ok(true)
        }
        "settings" => {
            output.line_stderr("[session] loading effective config")?;
            send_load_config(writer, state)?;
            Ok(true)
        }
        "feedback" => {
            let args = parts.map(str::to_string).collect::<Vec<_>>();
            let Some(parsed) = parse_feedback_args(&args) else {
                output.line_stderr(
                    "[session] usage: :feedback <bug|bad_result|good_result|safety_check|other> [reason] [--logs|--no-logs]",
                )?;
                return Ok(true);
            };
            let current_thread = state.thread_id.clone();
            output.line_stderr(format!(
                "[feedback] submitting {} feedback",
                summarize_text(&parsed.classification)
            ))?;
            send_feedback_upload(
                writer,
                state,
                parsed.classification,
                parsed.reason,
                current_thread,
                parsed.include_logs,
            )?;
            Ok(true)
        }
        "logout" => {
            output.line_stderr("[session] logging out")?;
            send_logout_account(writer, state)?;
            Ok(true)
        }
        "debug-config" => {
            output.line_stderr("[session] loading effective config")?;
            send_load_config(writer, state)?;
            Ok(true)
        }
        "fast"
        | "personality"
        | "collab"
        | "agent"
        | "multi-agents"
        | "theme"
        | "rollout"
        | "experimental"
        | "sandbox-add-read-dir"
        | "setup-default-sandbox"
        | "init" => {
            output.line_stderr(format!(
                "[session] /{command} is recognized, but this inline client does not yet implement the native Codex popup/workflow for it"
            ))?;
            Ok(true)
        }
        "realtime" => {
            output.line_stderr(
                "[session] /realtime is not implemented in codexw yet; app-server exposes realtime methods, but this client does not yet provide the audio/session UI needed to use them cleanly",
            )?;
            Ok(true)
        }
        "ps" => {
            output.line_stderr(
                "[session] /ps is not implemented in codexw because app-server exposes background terminal cleanup but not a background-terminal listing surface like the native TUI has internally; /clean is available",
            )?;
            Ok(true)
        }
        "plan" => {
            output.line_stderr(
                "[session] /plan is not implemented in codexw yet; native Codex switches collaboration mode internally, and app-server does not expose that mode control directly to this client",
            )?;
            Ok(true)
        }
        _ => {
            output.line_stderr(format!("[session] unknown command: {command}"))?;
            Ok(true)
        }
    }
}

struct FeedbackCommand {
    classification: String,
    reason: Option<String>,
    include_logs: bool,
}

fn parse_feedback_args(args: &[String]) -> Option<FeedbackCommand> {
    if args.is_empty() {
        return None;
    }
    let mut include_logs = false;
    let mut filtered = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--logs" => include_logs = true,
            "--no-logs" => include_logs = false,
            _ => filtered.push(arg.as_str()),
        }
    }
    let Some(first) = filtered.first() else {
        return None;
    };
    let classification = normalize_feedback_classification(first)?;
    let reason = join_prompt(
        &filtered[1..]
            .iter()
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>(),
    );
    Some(FeedbackCommand {
        classification,
        reason,
        include_logs,
    })
}

fn normalize_feedback_classification(raw: &str) -> Option<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bug" => Some("bug".to_string()),
        "bad" | "bad-result" | "bad_result" => Some("bad_result".to_string()),
        "good" | "good-result" | "good_result" => Some("good_result".to_string()),
        "safety" | "safety-check" | "safety_check" => Some("safety_check".to_string()),
        "other" => Some("other".to_string()),
        _ => None,
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

fn send_json<T: serde::Serialize>(writer: &mut ChildStdin, value: &T) -> Result<()> {
    let mut encoded = serde_json::to_string(value).context("serialize JSON-RPC message")?;
    encoded.push('\n');
    writer
        .write_all(encoded.as_bytes())
        .context("write JSON-RPC message")?;
    writer.flush().context("flush JSON-RPC message")
}

fn update_prompt(output: &mut Output, state: &AppState, editor: &LineEditor) -> Result<()> {
    let prompt = prompt_is_visible(state).then(String::new);
    let status = prompt_is_visible(state).then(|| render_prompt_status(state));
    output.set_prompt(prompt);
    output.set_status(status);
    output
        .show_prompt(editor.buffer(), editor.cursor_chars())
        .context("show prompt")
}

fn prompt_is_visible(state: &AppState) -> bool {
    state.thread_id.is_some() && !state.pending_thread_switch
}

fn prompt_accepts_input(state: &AppState) -> bool {
    prompt_is_visible(state) && state.active_exec_process_id.is_none()
}

fn handle_tab_completion(
    editor: &mut LineEditor,
    state: &AppState,
    resolved_cwd: &str,
    output: &mut Output,
) -> Result<()> {
    let buffer = editor.buffer().to_string();
    let cursor_byte = editor.cursor_byte_index();

    if let Some(result) = try_complete_slash_command(editor, &buffer, cursor_byte) {
        if let Some(rendered) = result.rendered_candidates {
            output.block_stdout("Command completions", &rendered)?;
        }
        return Ok(());
    }

    if let Some(result) = try_complete_file_token(editor, &buffer, cursor_byte, resolved_cwd)? {
        if let Some(rendered) = result.rendered_candidates {
            output.block_stdout("File completions", &rendered)?;
        }
        return Ok(());
    }

    if !state.turn_running && !buffer.trim_start().starts_with('!') {
        output.line_stderr("[tab] no completion available")?;
    }
    Ok(())
}

fn builtin_command_description(command: &str) -> &'static str {
    builtin_command_entry(command)
        .map(|entry| entry.description)
        .unwrap_or("command")
}

struct FileCompletionResult {
    rendered_candidates: Option<String>,
}

struct SlashCompletionResult {
    rendered_candidates: Option<String>,
}

fn try_complete_slash_command(
    editor: &mut LineEditor,
    buffer: &str,
    cursor_byte: usize,
) -> Option<SlashCompletionResult> {
    let Some((command_start, command_end, prefix)) = slash_command_at_cursor(buffer, cursor_byte)
    else {
        return None;
    };

    let mut prefix_matches = builtin_command_names()
        .iter()
        .copied()
        .filter(|name| name.starts_with(prefix))
        .collect::<Vec<_>>();

    if prefix_matches.is_empty() && prefix.is_empty() {
        prefix_matches = builtin_command_names().to_vec();
    }

    if prefix_matches.len() == 1 {
        editor.replace_range(
            command_start,
            command_end,
            &format!("/{} ", prefix_matches[0]),
        );
        return Some(SlashCompletionResult {
            rendered_candidates: None,
        });
    }

    if !prefix_matches.is_empty() {
        let lcp = longest_common_prefix(&prefix_matches);
        if lcp.len() > prefix.len() {
            editor.replace_range(command_start, command_end, &format!("/{lcp}"));
            return Some(SlashCompletionResult {
                rendered_candidates: None,
            });
        }

        return Some(SlashCompletionResult {
            rendered_candidates: Some(render_slash_completion_candidates(
                prefix,
                &prefix_matches,
                false,
            )),
        });
    }

    let mut fuzzy_matches = builtin_command_names()
        .iter()
        .filter_map(|name| fuzzy_match_score(name, prefix).map(|score| (*name, score)))
        .collect::<Vec<_>>();
    if fuzzy_matches.is_empty() {
        return None;
    }
    fuzzy_matches.sort_by(|(name_a, score_a), (name_b, score_b)| {
        score_a.cmp(score_b).then_with(|| name_a.cmp(name_b))
    });
    let fuzzy_names = fuzzy_matches
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    Some(SlashCompletionResult {
        rendered_candidates: Some(render_slash_completion_candidates(
            prefix,
            &fuzzy_names,
            true,
        )),
    })
}

fn render_slash_completion_candidates(filter: &str, matches: &[&str], fuzzy: bool) -> String {
    let mut lines = Vec::new();
    if filter.is_empty() {
        lines.push("Slash commands:".to_string());
    } else {
        lines.push(format!(
            "{} matches for /{}:",
            if fuzzy { "Fuzzy" } else { "Command" },
            filter
        ));
    }
    for (idx, name) in matches.iter().take(12).enumerate() {
        lines.push(format!(
            "{:>2}. /{:<16} {}",
            idx + 1,
            name,
            builtin_command_description(name)
        ));
    }
    if matches.len() > 12 {
        lines.push(format!("…and {} more", matches.len() - 12));
    }
    lines.join("\n")
}

fn fuzzy_match_score(haystack: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(i32::MAX);
    }

    let mut lowered_chars = Vec::new();
    let mut lowered_to_orig_char_idx = Vec::new();
    for (orig_idx, ch) in haystack.chars().enumerate() {
        for lc in ch.to_lowercase() {
            lowered_chars.push(lc);
            lowered_to_orig_char_idx.push(orig_idx);
        }
    }

    let lowered_needle = needle.to_lowercase().chars().collect::<Vec<_>>();
    let mut result_orig_indices = Vec::with_capacity(lowered_needle.len());
    let mut last_lower_pos = None;
    let mut cur = 0usize;
    for &nc in &lowered_needle {
        let mut found_at = None;
        while cur < lowered_chars.len() {
            if lowered_chars[cur] == nc {
                found_at = Some(cur);
                cur += 1;
                break;
            }
            cur += 1;
        }
        let pos = found_at?;
        result_orig_indices.push(lowered_to_orig_char_idx[pos]);
        last_lower_pos = Some(pos);
    }

    let first_lower_pos = if result_orig_indices.is_empty() {
        0usize
    } else {
        let target_orig = result_orig_indices[0];
        lowered_to_orig_char_idx
            .iter()
            .position(|&oi| oi == target_orig)
            .unwrap_or(0)
    };
    let last_lower_pos = last_lower_pos.unwrap_or(first_lower_pos);
    let window =
        (last_lower_pos as i32 - first_lower_pos as i32 + 1) - (lowered_needle.len() as i32);
    let mut score = window.max(0);
    if first_lower_pos == 0 {
        score -= 100;
    }
    Some(score)
}

fn try_complete_file_token(
    editor: &mut LineEditor,
    buffer: &str,
    cursor_byte: usize,
    resolved_cwd: &str,
) -> Result<Option<FileCompletionResult>> {
    let Some((start, end, token)) = current_at_token(buffer, cursor_byte) else {
        return Ok(None);
    };
    let completions = file_completions(&token, resolved_cwd)?;
    if completions.is_empty() {
        return Ok(None);
    }

    if completions.len() == 1 {
        editor.replace_range(start, end, &format!("{} ", completions[0]));
        return Ok(Some(FileCompletionResult {
            rendered_candidates: None,
        }));
    }

    let lcp = longest_common_prefix(&completions);
    let inserted_prefix = if lcp.len() > token.len() {
        &lcp
    } else {
        &token
    };
    editor.replace_range(start, end, &format!("@{inserted_prefix}"));
    let rendered_candidates = Some(
        completions
            .iter()
            .take(12)
            .enumerate()
            .map(|(idx, candidate)| format!("{:>2}. {}", idx + 1, candidate))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok(Some(FileCompletionResult {
        rendered_candidates,
    }))
}

fn slash_command_at_cursor<'a>(
    buffer: &'a str,
    cursor_byte: usize,
) -> Option<(usize, usize, &'a str)> {
    let first_line_end = buffer.find('\n').unwrap_or(buffer.len());
    if cursor_byte > first_line_end {
        return None;
    }
    let first_line = &buffer[..first_line_end];
    let Some(stripped) = first_line.strip_prefix('/') else {
        return None;
    };
    let name_end = stripped
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(stripped.len());
    let command_end = 1 + name_end;
    if cursor_byte > command_end {
        return None;
    }
    Some((0, command_end, &stripped[..name_end]))
}

fn current_at_token<'a>(buffer: &'a str, cursor_byte: usize) -> Option<(usize, usize, String)> {
    let safe_cursor = clamp_to_char_boundary(buffer, cursor_byte);
    let before_cursor = &buffer[..safe_cursor];
    let after_cursor = &buffer[safe_cursor..];
    let start = before_cursor
        .char_indices()
        .rfind(|(_, ch)| ch.is_whitespace())
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);
    let end_rel = after_cursor
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(after_cursor.len());
    let end = safe_cursor + end_rel;
    let token = &buffer[start..end];
    let mention = token.strip_prefix('@')?;
    if mention.is_empty() {
        return Some((start, end, String::new()));
    }
    if mention.starts_with('@') {
        return None;
    }
    if mention
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'))
    {
        return None;
    }
    Some((start, end, mention.to_string()))
}

fn clamp_to_char_boundary(text: &str, cursor_byte: usize) -> usize {
    if cursor_byte >= text.len() {
        return text.len();
    }
    let mut safe = cursor_byte;
    while safe > 0 && !text.is_char_boundary(safe) {
        safe -= 1;
    }
    safe
}

fn file_completions(token: &str, resolved_cwd: &str) -> Result<Vec<String>> {
    let token = token.trim();
    let (dir_part, name_prefix) = match token.rfind(['/', '\\']) {
        Some(idx) => (&token[..=idx], &token[idx + 1..]),
        None => ("", token),
    };
    let base_dir = if dir_part.is_empty() {
        PathBuf::from(resolved_cwd)
    } else {
        PathBuf::from(resolved_cwd).join(dir_part)
    };
    if !base_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut matches = std::fs::read_dir(&base_dir)
        .with_context(|| format!("read directory {}", base_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = os_str_to_string(&name)?;
            if !name.starts_with(name_prefix) {
                return None;
            }
            let mut rendered = format!("{dir_part}{name}");
            if entry.path().is_dir() {
                rendered.push('/');
            }
            Some(rendered)
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches)
}

fn os_str_to_string(value: &OsStr) -> Option<String> {
    value.to_str().map(ToOwned::to_owned)
}

fn quote_if_needed(value: &str) -> String {
    if value.chars().any(char::is_whitespace) && !value.contains('"') {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

fn longest_common_prefix<S: AsRef<str>>(values: &[S]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let mut prefix = values[0].as_ref().to_string();
    for value in &values[1..] {
        let mut next = String::new();
        for (a, b) in prefix.chars().zip(value.as_ref().chars()) {
            if a != b {
                break;
            }
            next.push(a);
        }
        prefix = next;
        if prefix.is_empty() {
            break;
        }
    }
    prefix
}

#[derive(Clone, Copy)]
struct BuiltinCommandEntry {
    name: &'static str,
    help_syntax: &'static str,
    description: &'static str,
}

fn builtin_command_entry(command: &str) -> Option<&'static BuiltinCommandEntry> {
    builtin_command_entries()
        .iter()
        .find(|entry| entry.name == command)
}

fn builtin_command_names() -> &'static [&'static str] {
    const NAMES: &[&str] = &[
        "model",
        "models",
        "fast",
        "approvals",
        "permissions",
        "setup-default-sandbox",
        "sandbox-add-read-dir",
        "experimental",
        "skills",
        "review",
        "rename",
        "new",
        "resume",
        "fork",
        "init",
        "compact",
        "plan",
        "collab",
        "agent",
        "multi-agents",
        "diff",
        "copy",
        "mention",
        "status",
        "debug-config",
        "statusline",
        "theme",
        "mcp",
        "apps",
        "logout",
        "quit",
        "exit",
        "feedback",
        "rollout",
        "ps",
        "clean",
        "clear",
        "personality",
        "realtime",
        "settings",
        "threads",
        "auto",
        "attach-image",
        "attach",
        "attach-url",
        "attachments",
        "clear-attachments",
        "interrupt",
        "help",
    ];
    NAMES
}

fn builtin_help_lines() -> Vec<String> {
    builtin_command_entries()
        .iter()
        .map(|entry| format!(":{:<26} {}", entry.help_syntax, entry.description))
        .collect()
}

fn builtin_command_entries() -> &'static [BuiltinCommandEntry] {
    const ENTRIES: &[BuiltinCommandEntry] = &[
        BuiltinCommandEntry {
            name: "model",
            help_syntax: "model",
            description: "choose what model and reasoning effort to use",
        },
        BuiltinCommandEntry {
            name: "models",
            help_syntax: "models",
            description: "list available models",
        },
        BuiltinCommandEntry {
            name: "fast",
            help_syntax: "fast",
            description: "toggle Fast mode to enable fastest inference at 2X plan usage",
        },
        BuiltinCommandEntry {
            name: "approvals",
            help_syntax: "approvals or /permissions",
            description: "show automation and permission posture",
        },
        BuiltinCommandEntry {
            name: "permissions",
            help_syntax: "permissions or /approvals",
            description: "show automation and permission posture",
        },
        BuiltinCommandEntry {
            name: "setup-default-sandbox",
            help_syntax: "setup-default-sandbox",
            description: "native sandbox setup workflow not yet ported",
        },
        BuiltinCommandEntry {
            name: "sandbox-add-read-dir",
            help_syntax: "sandbox-add-read-dir",
            description: "native sandbox read-dir workflow not yet ported",
        },
        BuiltinCommandEntry {
            name: "experimental",
            help_syntax: "experimental",
            description: "toggle experimental features",
        },
        BuiltinCommandEntry {
            name: "skills",
            help_syntax: "skills",
            description: "use skills to improve how Codex performs specific tasks",
        },
        BuiltinCommandEntry {
            name: "review",
            help_syntax: "review [instructions]",
            description: "review current changes and find issues",
        },
        BuiltinCommandEntry {
            name: "rename",
            help_syntax: "rename <name>",
            description: "rename the current thread",
        },
        BuiltinCommandEntry {
            name: "new",
            help_syntax: "new",
            description: "start a new thread",
        },
        BuiltinCommandEntry {
            name: "resume",
            help_syntax: "resume [thread-id|n]",
            description: "resume a saved thread",
        },
        BuiltinCommandEntry {
            name: "fork",
            help_syntax: "fork",
            description: "fork the current thread",
        },
        BuiltinCommandEntry {
            name: "init",
            help_syntax: "init",
            description: "create an AGENTS.md file with instructions for Codex",
        },
        BuiltinCommandEntry {
            name: "compact",
            help_syntax: "compact",
            description: "summarize conversation to prevent hitting the context limit",
        },
        BuiltinCommandEntry {
            name: "plan",
            help_syntax: "plan",
            description: "switch to plan mode",
        },
        BuiltinCommandEntry {
            name: "collab",
            help_syntax: "collab",
            description: "change collaboration mode",
        },
        BuiltinCommandEntry {
            name: "agent",
            help_syntax: "agent",
            description: "switch the active agent thread",
        },
        BuiltinCommandEntry {
            name: "multi-agents",
            help_syntax: "multi-agents",
            description: "switch the active agent thread",
        },
        BuiltinCommandEntry {
            name: "diff",
            help_syntax: "diff",
            description: "show the latest turn diff snapshot",
        },
        BuiltinCommandEntry {
            name: "copy",
            help_syntax: "copy",
            description: "copy the latest assistant reply",
        },
        BuiltinCommandEntry {
            name: "mention",
            help_syntax: "mention [query|n]",
            description: "insert or search mentionable files",
        },
        BuiltinCommandEntry {
            name: "status",
            help_syntax: "status",
            description: "show current session configuration and token usage",
        },
        BuiltinCommandEntry {
            name: "debug-config",
            help_syntax: "debug-config",
            description: "show config layers and requirement sources for debugging",
        },
        BuiltinCommandEntry {
            name: "statusline",
            help_syntax: "statusline",
            description: "show current session status",
        },
        BuiltinCommandEntry {
            name: "theme",
            help_syntax: "theme",
            description: "choose a syntax highlighting theme",
        },
        BuiltinCommandEntry {
            name: "mcp",
            help_syntax: "mcp",
            description: "list MCP servers and tools",
        },
        BuiltinCommandEntry {
            name: "apps",
            help_syntax: "apps",
            description: "list known app mentions",
        },
        BuiltinCommandEntry {
            name: "logout",
            help_syntax: "logout",
            description: "log out of Codex",
        },
        BuiltinCommandEntry {
            name: "quit",
            help_syntax: "quit",
            description: "exit CodexW",
        },
        BuiltinCommandEntry {
            name: "exit",
            help_syntax: "exit",
            description: "exit CodexW",
        },
        BuiltinCommandEntry {
            name: "feedback",
            help_syntax: "feedback <category> [reason] [--logs|--no-logs]",
            description: "submit feedback through app-server",
        },
        BuiltinCommandEntry {
            name: "rollout",
            help_syntax: "rollout",
            description: "native rollout-path display not yet ported",
        },
        BuiltinCommandEntry {
            name: "ps",
            help_syntax: "ps",
            description: "list background terminals",
        },
        BuiltinCommandEntry {
            name: "clean",
            help_syntax: "clean",
            description: "stop background terminals for the thread",
        },
        BuiltinCommandEntry {
            name: "clear",
            help_syntax: "clear",
            description: "clear terminal and start a new thread",
        },
        BuiltinCommandEntry {
            name: "personality",
            help_syntax: "personality",
            description: "choose a communication style for Codex",
        },
        BuiltinCommandEntry {
            name: "realtime",
            help_syntax: "realtime",
            description: "experimental realtime workflow",
        },
        BuiltinCommandEntry {
            name: "settings",
            help_syntax: "settings",
            description: "show effective backend config",
        },
        BuiltinCommandEntry {
            name: "threads",
            help_syntax: "threads [query]",
            description: "list recent threads",
        },
        BuiltinCommandEntry {
            name: "auto",
            help_syntax: "auto on|off",
            description: "toggle auto-continue",
        },
        BuiltinCommandEntry {
            name: "attach-image",
            help_syntax: "attach-image <path>",
            description: "queue a local image for next submit",
        },
        BuiltinCommandEntry {
            name: "attach",
            help_syntax: "attach <path>",
            description: "queue a local image for next submit",
        },
        BuiltinCommandEntry {
            name: "attach-url",
            help_syntax: "attach-url <url>",
            description: "queue a remote image for next submit",
        },
        BuiltinCommandEntry {
            name: "attachments",
            help_syntax: "attachments",
            description: "show queued attachments",
        },
        BuiltinCommandEntry {
            name: "clear-attachments",
            help_syntax: "clear-attachments",
            description: "clear queued attachments",
        },
        BuiltinCommandEntry {
            name: "interrupt",
            help_syntax: "interrupt",
            description: "interrupt the active turn or command",
        },
        BuiltinCommandEntry {
            name: "help",
            help_syntax: "help",
            description: "show commands",
        },
    ];
    ENTRIES
}

fn render_prompt_status(state: &AppState) -> String {
    let detail = state
        .last_status_line
        .as_deref()
        .filter(|line| !line.trim().is_empty() && *line != "ready");
    if state.active_exec_process_id.is_some() {
        if let Some(detail) = detail {
            format!(
                "{} {} · {}",
                spinner_frame(state.activity_started_at),
                detail,
                format_elapsed(state.activity_started_at),
            )
        } else {
            format!(
                "{} cmd · {}",
                spinner_frame(state.activity_started_at),
                format_elapsed(state.activity_started_at),
            )
        }
    } else if state.turn_running {
        if let Some(detail) = detail {
            format!(
                "{} {} · {}",
                spinner_frame(state.activity_started_at),
                detail,
                format_elapsed(state.activity_started_at),
            )
        } else {
            format!(
                "{} turn {} · {}",
                spinner_frame(state.activity_started_at),
                state.started_turn_count.max(1),
                format_elapsed(state.activity_started_at)
            )
        }
    } else {
        format!("ready · {} turns", state.completed_turn_count)
    }
}

fn spinner_frame(started_at: Option<Instant>) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let idx = started_at
        .map(|start| {
            ((Instant::now().saturating_duration_since(start).as_millis() / 100) as usize)
                % FRAMES.len()
        })
        .unwrap_or(0);
    FRAMES[idx]
}

fn format_elapsed(started_at: Option<Instant>) -> String {
    let elapsed = started_at
        .map(|start| Instant::now().saturating_duration_since(start).as_secs())
        .unwrap_or(0);
    if elapsed < 60 {
        format!("{elapsed}s")
    } else {
        format!("{}m{:02}s", elapsed / 60, elapsed % 60)
    }
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

fn join_prompt(parts: &[String]) -> Option<String> {
    let joined = parts.join(" ").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn is_builtin_command(command_line: &str) -> bool {
    let command = command_line.split_whitespace().next().unwrap_or_default();
    matches!(command, "h" | "q") || builtin_command_names().contains(&command)
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

fn render_command_completion(
    command: &str,
    status: &str,
    exit_code: &str,
    output: Option<&str>,
) -> String {
    let mut rendered = format!("{command}\n[status] {status}  [exit] {exit_code}");
    if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
            rendered.push_str(trimmed);
        }
    }
    rendered
}

fn render_local_command_completion(
    command: &str,
    exit_code: &str,
    stdout: &str,
    stderr: &str,
) -> String {
    let mut rendered = format!("{command}\n[exit] {exit_code}");
    if !stdout.trim().is_empty() {
        rendered.push_str("\n\n[stdout]\n");
        rendered.push_str(stdout.trim_end());
    }
    if !stderr.trim().is_empty() {
        rendered.push_str("\n\n[stderr]\n");
        rendered.push_str(stderr.trim_end());
    }
    rendered
}

fn render_file_change_completion(item: &Value, status: &str, output: Option<&str>) -> String {
    let mut rendered = format!("[status] {status}\n{}", summarize_file_change_paths(item));
    let structured = render_file_changes(item);
    if !structured.is_empty() {
        rendered.push_str("\n\n");
        rendered.push_str(&structured);
    } else if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
            rendered.push_str(trimmed);
        }
    }
    rendered
}

fn render_pending_attachments(local_images: &[String], remote_images: &[String]) -> String {
    let mut lines = Vec::new();
    for path in local_images {
        lines.push(format!("local-image  {path}"));
    }
    for url in remote_images {
        lines.push(format!("remote-image {url}"));
    }
    lines.join("\n")
}

fn render_status_snapshot(cli: &Cli, resolved_cwd: &str, state: &AppState) -> String {
    let mut lines = vec![
        format!("cwd             {resolved_cwd}"),
        format!(
            "thread          {}",
            state.thread_id.as_deref().unwrap_or("-")
        ),
        format!(
            "turn            {}",
            state.active_turn_id.as_deref().unwrap_or("-")
        ),
        format!(
            "turn count      started={} completed={}",
            state.started_turn_count, state.completed_turn_count
        ),
        format!("running         {}", state.turn_running),
        format!(
            "local command   {}",
            state.active_exec_process_id.as_deref().unwrap_or("-")
        ),
        format!("auto-continue   {}", state.auto_continue),
        format!("approval        {}", approval_policy(cli)),
        format!("sandbox(thread) {}", thread_sandbox_mode(cli)),
        format!(
            "sandbox(turn)   {}",
            summarize_sandbox_policy(&turn_sandbox_policy(cli))
        ),
        format!(
            "model           {}",
            cli.model.as_deref().unwrap_or("default")
        ),
        format!(
            "provider        {}",
            cli.model_provider.as_deref().unwrap_or("default")
        ),
        format!(
            "objective       {}",
            summarize_text(state.objective.as_deref().unwrap_or("-"))
        ),
        format!(
            "attachments     local={} remote={}",
            state.pending_local_images.len(),
            state.pending_remote_images.len()
        ),
        format!(
            "mentions        apps={} plugins={} skills={}",
            state.apps.iter().filter(|entry| entry.enabled).count(),
            state.plugins.iter().filter(|entry| entry.enabled).count(),
            state.skills.iter().filter(|entry| entry.enabled).count(),
        ),
    ];

    if let Some(account) = render_account_summary(state.account_info.as_ref()) {
        lines.push(format!("account         {account}"));
    }
    if state.turn_running || state.active_exec_process_id.is_some() {
        lines.push(format!(
            "active time     {}",
            format_elapsed(state.activity_started_at)
        ));
    }
    lines.extend(render_rate_limit_lines(state.rate_limits.as_ref()));
    if let Some(token_usage) = render_token_usage_summary(state.last_token_usage.as_ref()) {
        lines.push(format!("tokens          {token_usage}"));
    }
    if let Some(last_status) = state.last_status_line.as_deref() {
        lines.push(format!("status          {last_status}"));
    }
    if let Some(last_message) = state.last_agent_message.as_deref() {
        lines.push(format!("last reply      {}", summarize_text(last_message)));
    }
    if let Some(diff) = state.last_turn_diff.as_deref() {
        lines.push(format!("diff            {} chars", diff.chars().count()));
    }

    lines.join("\n")
}

fn render_permissions_snapshot(cli: &Cli) -> String {
    [
        format!("approval policy  {}", approval_policy(cli)),
        format!("thread sandbox   {}", thread_sandbox_mode(cli)),
        format!(
            "turn sandbox     {}",
            summarize_sandbox_policy(&turn_sandbox_policy(cli))
        ),
        "network access    enabled".to_string(),
        "tool use          automatic".to_string(),
        "shell exec        automatic".to_string(),
        "host access       full".to_string(),
    ]
    .join("\n")
}

fn render_config_snapshot(result: &Value) -> String {
    if result.is_null() {
        return "config unavailable".to_string();
    }
    serde_json::to_string_pretty(result).unwrap_or_else(|_| summarize_value(result))
}

fn copy_to_clipboard(text: &str, output: &mut Output) -> Result<()> {
    if cfg!(target_os = "macos") {
        let Ok(mut child) = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            output.block_stdout("Copied text", text)?;
            return Ok(());
        };
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .context("write pbcopy input")?;
        }
        let status = child.wait().context("wait for pbcopy")?;
        if status.success() {
            output.line_stderr("[copy] latest assistant reply copied to clipboard")?;
            return Ok(());
        }
    }
    output.block_stdout("Copied text", text)?;
    Ok(())
}

fn render_resumed_history(result: &Value, state: &mut AppState, output: &mut Output) -> Result<()> {
    let turns = result
        .get("thread")
        .and_then(|thread| thread.get("turns"))
        .and_then(Value::as_array);
    let Some(turns) = turns else {
        return Ok(());
    };
    if turns.is_empty() {
        return Ok(());
    }

    seed_resumed_state_from_turns(turns, state);
    let conversation_items = latest_conversation_history_items(turns, 10);
    if conversation_items.is_empty() {
        return Ok(());
    }

    output.line_stderr("[history] showing latest 10 conversation messages from resumed thread")?;
    for item in conversation_items {
        render_history_item(item, state, output)?;
    }
    Ok(())
}

fn latest_conversation_history_items<'a>(turns: &'a [Value], limit: usize) -> Vec<&'a Value> {
    let mut items = Vec::with_capacity(limit);
    for turn in turns.iter().rev() {
        if let Some(turn_items) = turn.get("items").and_then(Value::as_array) {
            for item in turn_items.iter().rev() {
                if is_conversation_history_item(item) {
                    items.push(item);
                    if items.len() == limit {
                        items.reverse();
                        return items;
                    }
                }
            }
        }
    }
    items.reverse();
    items
}

fn is_conversation_history_item(item: &Value) -> bool {
    match get_string(item, &["type"]).unwrap_or("") {
        "userMessage" => item
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|content| !content.is_empty()),
        "agentMessage" => item
            .get("text")
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty()),
        _ => false,
    }
}

fn seed_resumed_state_from_turns(turns: &[Value], state: &mut AppState) {
    let mut latest_user_message = None;
    let mut latest_agent_message = None;

    'outer: for turn in turns.iter().rev() {
        if let Some(items) = turn.get("items").and_then(Value::as_array) {
            for item in items.iter().rev() {
                match get_string(item, &["type"]).unwrap_or("") {
                    "userMessage" if latest_user_message.is_none() => {
                        if let Some(content) = item.get("content").and_then(Value::as_array) {
                            let rendered = render_user_message_history(content);
                            if !rendered.trim().is_empty() {
                                latest_user_message = Some(rendered);
                            }
                        }
                    }
                    "agentMessage" if latest_agent_message.is_none() => {
                        let text = sanitize_history_text(get_string(item, &["text"]).unwrap_or(""));
                        if !text.trim().is_empty() {
                            latest_agent_message = Some(text);
                        }
                    }
                    _ => {}
                }

                if latest_user_message.is_some() && latest_agent_message.is_some() {
                    break 'outer;
                }
            }
        }
    }

    if let Some(message) = latest_user_message {
        state.objective = Some(message);
    }
    if let Some(message) = latest_agent_message {
        state.last_agent_message = Some(message);
    }
}

fn render_history_item(item: &Value, state: &mut AppState, output: &mut Output) -> Result<()> {
    match get_string(item, &["type"]).unwrap_or("") {
        "userMessage" => {
            let content = item
                .get("content")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let rendered = render_user_message_history(&content);
            if !rendered.trim().is_empty() {
                output.block_stdout("User", &rendered)?;
            }
        }
        "agentMessage" => {
            let text = sanitize_history_text(get_string(item, &["text"]).unwrap_or(""));
            if !text.trim().is_empty() {
                state.last_agent_message = Some(text.clone());
                output.block_stdout("Assistant", &text)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_user_message_history(content: &[Value]) -> String {
    let mut parts = Vec::new();
    for item in content {
        match get_string(item, &["type"]).unwrap_or("") {
            "text" => {
                if let Some(text) = get_string(item, &["text"]) {
                    parts.push(text.to_string());
                }
            }
            "image" => {
                if let Some(url) = get_string(item, &["imageUrl"]) {
                    parts.push(format!("[image] {url}"));
                }
            }
            "localImage" => {
                if let Some(path) = get_string(item, &["path"]) {
                    parts.push(format!("[local-image] {path}"));
                }
            }
            "mention" => {
                let label = get_string(item, &["label"]).unwrap_or("$mention");
                let uri = get_string(item, &["uri"]).unwrap_or("");
                if uri.is_empty() {
                    parts.push(label.to_string());
                } else {
                    parts.push(format!("{label} ({uri})"));
                }
            }
            "skill" => {
                if let Some(path) = get_string(item, &["path"]) {
                    parts.push(format!("[skill] {path}"));
                }
            }
            _ => {}
        }
    }
    sanitize_history_text(&parts.join("\n"))
}

fn sanitize_history_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let min_indent = lines
        .iter()
        .filter_map(|line| {
            if line.trim().is_empty() {
                None
            } else {
                Some(
                    line.chars()
                        .take_while(|ch| *ch == ' ' || *ch == '\t')
                        .count(),
                )
            }
        })
        .min()
        .unwrap_or(0);
    let cleaned = lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.chars().skip(min_indent).collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    cleaned
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn summarize_sandbox_policy(policy: &Value) -> String {
    match get_string(policy, &["type"]).unwrap_or("unknown") {
        "dangerFullAccess" => "dangerFullAccess".to_string(),
        other => summarize_value(&json!({
            "type": other,
            "policy": policy,
        })),
    }
}

fn render_account_summary(account: Option<&Value>) -> Option<String> {
    let account = account?;
    if account.is_null() {
        return Some("not signed in".to_string());
    }
    let account_type = get_string(account, &["type"])
        .or_else(|| get_string(account, &["authMode"]))
        .unwrap_or("unknown");
    let mut parts = vec![account_type.to_string()];
    if let Some(email) = get_string(account, &["email"]) {
        parts.push(email.to_string());
    }
    if let Some(plan_type) = get_string(account, &["planType"]) {
        parts.push(format!("plan={plan_type}"));
    }
    Some(parts.join(" "))
}

fn render_rate_limit_lines(rate_limits: Option<&Value>) -> Vec<String> {
    let Some(rate_limits) = rate_limits else {
        return vec!["rate limits     unavailable".to_string()];
    };

    let mut lines = Vec::new();
    let mut first_row = true;
    for (label, window_key) in [("primary", "primary"), ("secondary", "secondary")] {
        let Some(window) = rate_limits.get(window_key) else {
            continue;
        };
        if window.is_null() {
            continue;
        }
        let used_percent = window
            .get("usedPercent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let percent_left = (100.0 - used_percent).clamp(0.0, 100.0);
        let window_minutes = window.get("windowDurationMins").and_then(Value::as_i64);
        let duration_label = window_minutes
            .map(get_limits_duration)
            .unwrap_or_else(|| label.to_string());
        let reset_label = window
            .get("resetsAt")
            .and_then(Value::as_i64)
            .and_then(format_reset_timestamp_local);
        let mut line = format!(
            "{}{} limit {}",
            if first_row {
                "rate limits     "
            } else {
                "                "
            },
            duration_label,
            format_status_limit_summary(percent_left),
        );
        if let Some(reset_label) = reset_label {
            line.push_str(&format!(" (resets {reset_label})"));
        }
        lines.push(line);
        first_row = false;
    }

    if let Some(credits) = rate_limits.get("credits")
        && let Some(credit_line) = render_credit_line(credits, first_row)
    {
        lines.push(credit_line);
    }

    if lines.is_empty() {
        vec!["rate limits     none reported".to_string()]
    } else {
        lines
    }
}

fn render_credit_line(credits: &Value, first_row: bool) -> Option<String> {
    if !credits
        .get("hasCredits")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let prefix = if first_row {
        "rate limits     "
    } else {
        "                "
    };
    if credits
        .get("unlimited")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Some(format!("{prefix}credits unlimited"));
    }
    let balance = credits.get("balance").and_then(Value::as_str)?.trim();
    if balance.is_empty() {
        return None;
    }
    Some(format!("{prefix}credits {balance}"))
}

fn render_fuzzy_file_search_results(query: &str, files: &[Value]) -> String {
    if files.is_empty() {
        return format!("No files matched \"{query}\".");
    }
    let mut lines = vec![format!("Query: {query}")];
    for (index, file) in files.iter().take(20).enumerate() {
        let path = get_string(file, &["path"]).unwrap_or("?");
        let score = file
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        lines.push(format!("{:>2}. {}  [score {}]", index + 1, path, score));
    }
    if files.len() > 20 {
        lines.push(format!("…and {} more", files.len() - 20));
    }
    lines.push("Use /mention <n> to insert a match into the prompt.".to_string());
    lines.join("\n")
}

fn render_apps_list(apps: &[AppCatalogEntry]) -> String {
    if apps.is_empty() {
        return "No apps are currently available.".to_string();
    }
    apps.iter()
        .map(|app| {
            format!(
                "{}  ${}  [{}]",
                app.name,
                app.slug,
                if app.enabled { "enabled" } else { "disabled" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_skills_list(skills: &[SkillCatalogEntry]) -> String {
    if skills.is_empty() {
        return "No skills found for the current workspace.".to_string();
    }
    skills
        .iter()
        .map(|skill| {
            format!(
                "{}  {}  [{}]",
                skill.name,
                skill.path,
                if skill.enabled { "enabled" } else { "disabled" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_models_list(result: &Value) -> String {
    let models = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if models.is_empty() {
        return "No models returned by app-server.".to_string();
    }
    models
        .iter()
        .take(30)
        .map(|model| {
            let id = get_string(model, &["id"])
                .or_else(|| get_string(model, &["model"]))
                .unwrap_or("?");
            let provider = get_string(model, &["provider"])
                .or_else(|| get_string(model, &["modelProvider"]))
                .unwrap_or("default");
            let effort = model
                .get("reasoningEffortOptions")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "-".to_string());
            format!("{id}  [provider {provider}]  [effort {effort}]")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_mcp_server_list(result: &Value) -> String {
    let entries = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if entries.is_empty() {
        return "No MCP servers returned by app-server.".to_string();
    }
    entries
        .iter()
        .map(|entry| {
            let name = get_string(entry, &["name"]).unwrap_or("?");
            let auth = get_string(entry, &["authStatus"])
                .or_else(|| get_string(entry, &["auth", "status"]))
                .unwrap_or("unknown");
            let tools = entry
                .get("tools")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);
            let resources = entry
                .get("resources")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);
            format!("{name}  [auth {auth}]  [tools {tools}]  [resources {resources}]")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_thread_list(result: &Value, search_term: Option<&str>) -> String {
    let threads = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if threads.is_empty() {
        return match search_term {
            Some(search_term) => format!("No threads matched \"{search_term}\"."),
            None => "No threads found for the current workspace.".to_string(),
        };
    }
    let mut lines = Vec::new();
    if let Some(search_term) = search_term {
        lines.push(format!("Search: {search_term}"));
    }
    lines.extend(threads.iter().enumerate().map(|(index, thread)| {
        let id = get_string(thread, &["id"]).unwrap_or("?");
        let preview = get_string(thread, &["preview"]).unwrap_or("-");
        let status = get_string(thread, &["status", "type"]).unwrap_or("unknown");
        let updated_at = thread
            .get("updatedAt")
            .and_then(Value::as_i64)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "{:>2}. {id}  [{status}]  [updated {updated_at}]  {}",
            index + 1,
            summarize_text(preview)
        )
    }));
    lines.push("Use /resume <n> to resume one of these threads.".to_string());
    lines.join("\n")
}

fn extract_thread_ids(result: &Value) -> Vec<String> {
    result
        .get("data")
        .and_then(Value::as_array)
        .map(|threads| {
            threads
                .iter()
                .filter_map(|thread| get_string(thread, &["id"]).map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_file_search_paths(files: &[Value]) -> Vec<String> {
    files
        .iter()
        .filter_map(|file| get_string(file, &["path"]).map(ToOwned::to_owned))
        .collect()
}

fn render_token_usage_summary(token_usage: Option<&Value>) -> Option<String> {
    let token_usage = token_usage?;
    let last_total = token_usage
        .get("last")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    let cumulative_total = token_usage
        .get("total")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    match (last_total, cumulative_total) {
        (Some(last_total), Some(cumulative_total)) => {
            Some(format!("last={} total={}", last_total, cumulative_total))
        }
        (Some(last_total), None) => Some(format!("last={last_total}")),
        (None, Some(cumulative_total)) => Some(format!("total={cumulative_total}")),
        (None, None) => None,
    }
}

fn get_limits_duration(window_minutes: i64) -> String {
    const MINUTES_PER_HOUR: i64 = 60;
    const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
    const MINUTES_PER_WEEK: i64 = 7 * MINUTES_PER_DAY;
    const MINUTES_PER_MONTH: i64 = 30 * MINUTES_PER_DAY;
    const ROUNDING_BIAS_MINUTES: i64 = 3;

    let window_minutes = window_minutes.max(0);
    if window_minutes <= MINUTES_PER_DAY.saturating_add(ROUNDING_BIAS_MINUTES) {
        let adjusted = window_minutes.saturating_add(ROUNDING_BIAS_MINUTES);
        let hours = std::cmp::max(1, adjusted / MINUTES_PER_HOUR);
        format!("{hours}h")
    } else if window_minutes <= MINUTES_PER_WEEK.saturating_add(ROUNDING_BIAS_MINUTES) {
        "weekly".to_string()
    } else if window_minutes <= MINUTES_PER_MONTH.saturating_add(ROUNDING_BIAS_MINUTES) {
        "monthly".to_string()
    } else {
        "annual".to_string()
    }
}

fn format_status_limit_summary(percent_remaining: f64) -> String {
    format!("{percent_remaining:.0}% left")
}

fn format_reset_timestamp_local(unix_seconds: i64) -> Option<String> {
    let dt_utc = DateTime::<Utc>::from_timestamp(unix_seconds, 0)?;
    let dt_local = dt_utc.with_timezone(&Local);
    let now = Local::now();
    let time = dt_local.format("%H:%M").to_string();
    if dt_local.date_naive() == now.date_naive() {
        Some(time)
    } else {
        Some(format!("{time} on {}", dt_local.format("%-d %b")))
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

fn format_plan(params: &Value) -> String {
    params
        .get("plan")
        .and_then(Value::as_array)
        .map(|plan| {
            plan.iter()
                .map(|step| {
                    let step_text = get_string(step, &["step"]).unwrap_or("-");
                    let status = get_string(step, &["status"]).unwrap_or("pending");
                    format!("- [{status}] {step_text}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn render_file_changes(item: &Value) -> String {
    let Some(changes) = item.get("changes").and_then(Value::as_array) else {
        return String::new();
    };
    let mut rendered = String::new();
    for (idx, change) in changes.iter().enumerate() {
        if idx > 0 {
            rendered.push_str("\n\n");
        }
        let path = get_string(change, &["path"]).unwrap_or("?");
        let kind = get_string(change, &["kind"]).unwrap_or("?");
        rendered.push_str(&format!("{kind} {path}"));
        if let Some(diff) = get_string(change, &["diff"]) {
            if !diff.is_empty() {
                rendered.push_str("\n\n");
                rendered.push_str(diff);
            }
        }
    }
    rendered
}

fn summarize_file_change_paths(item: &Value) -> String {
    let Some(changes) = item.get("changes").and_then(Value::as_array) else {
        return "updating files".to_string();
    };
    let paths = changes
        .iter()
        .filter_map(|change| get_string(change, &["path"]))
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return "updating files".to_string();
    }
    let preview = paths.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    if paths.len() <= 3 {
        format!("updating {}", preview)
    } else {
        format!("updating {} and {} more", preview, paths.len() - 3)
    }
}

fn render_reasoning_item(item: &Value) -> String {
    let summary = item
        .get("summary")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !summary.is_empty() {
        return summary.join("\n\n");
    }

    item.get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

fn build_tool_user_input_response(params: &Value) -> Value {
    let mut answers = serde_json::Map::new();
    if let Some(questions) = params.get("questions").and_then(Value::as_array) {
        for question in questions {
            let Some(id) = get_string(question, &["id"]) else {
                continue;
            };
            let selected = question
                .get("options")
                .and_then(Value::as_array)
                .and_then(|options| options.first())
                .and_then(|first| get_string(first, &["label"]))
                .map(|label| vec![label.to_string()])
                .unwrap_or_else(|| vec!["".to_string()]);
            answers.insert(id.to_string(), json!({ "answers": selected }));
        }
    }
    Value::Object(
        [("answers".to_string(), Value::Object(answers))]
            .into_iter()
            .collect(),
    )
}

fn summarize_command_approval_request(params: &Value, decision: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(reason) = get_string(params, &["reason"]) {
        parts.push(format!("reason={reason}"));
    }
    if let Some(command) = get_string(params, &["command"]) {
        parts.push(format!("command={command}"));
    }
    if let Some(cwd) = get_string(params, &["cwd"]) {
        parts.push(format!("cwd={cwd}"));
    }
    if let Some(host) = get_string(params, &["networkApprovalContext", "host"]) {
        parts.push(format!("network_host={host}"));
    }
    parts.push(format!("decision={}", summarize_value(decision)));
    parts.join(" ")
}

fn summarize_generic_approval_request(params: &Value, method: &str) -> String {
    let mut parts = vec![method.to_string()];
    if let Some(reason) = get_string(params, &["reason"]) {
        parts.push(format!("reason={reason}"));
    }
    if let Some(root) = get_string(params, &["grantRoot"]) {
        parts.push(format!("grant_root={root}"));
    }
    if let Some(cwd) = get_string(params, &["cwd"]) {
        parts.push(format!("cwd={cwd}"));
    }
    parts.join(" ")
}

fn summarize_tool_request(params: &Value) -> String {
    if let Some(message) = get_string(params, &["message"]) {
        return message.to_string();
    }
    if let Some(questions) = params.get("questions").and_then(Value::as_array) {
        let rendered = questions
            .iter()
            .filter_map(|question| get_string(question, &["question"]))
            .collect::<Vec<_>>();
        if !rendered.is_empty() {
            return rendered.join(" | ");
        }
    }
    summarize_value(params)
}

fn summarize_thread_status_for_display(params: &Value) -> Option<String> {
    let status_type = get_string(params, &["status", "type"]).unwrap_or("unknown");
    let flags = params
        .get("status")
        .and_then(|v| v.get("activeFlags"))
        .and_then(Value::as_array)
        .map(|flags| flags.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    if status_type == "active" && flags.is_empty() {
        return None;
    }

    if flags.iter().any(|flag| *flag == "waitingOnApproval") {
        return Some("waiting on approval".to_string());
    }

    if flags.is_empty() {
        if status_type == "idle" {
            Some("ready".to_string())
        } else {
            Some(status_type.to_string())
        }
    } else {
        Some(flags.join(", "))
    }
}

fn summarize_model_reroute(params: &Value) -> String {
    let from_model = get_string(params, &["fromModel"]).unwrap_or("?");
    let to_model = get_string(params, &["toModel"]).unwrap_or("?");
    let reason = get_string(params, &["reason"]).unwrap_or("unspecified");
    format!("{from_model} -> {to_model} reason={reason}")
}

fn summarize_terminal_interaction(params: &Value) -> Option<String> {
    let process_id = get_string(params, &["processId"]).unwrap_or("?");
    let stdin = get_string(params, &["stdin"]).unwrap_or("").trim();
    if stdin.is_empty() {
        return None;
    }
    Some(format!(
        "process={process_id} stdin={}",
        summarize_text(stdin)
    ))
}

fn summarize_server_request_resolved(params: &Value) -> String {
    let thread_id = get_string(params, &["threadId"]).unwrap_or("?");
    let request_id = params
        .get("requestId")
        .map(summarize_value)
        .unwrap_or_else(|| "?".to_string());
    format!("thread={thread_id} request={request_id}")
}

fn humanize_item_type(item_type: &str) -> String {
    match item_type {
        "mcpToolCall" => "mcp-tool".to_string(),
        "dynamicToolCall" => "dynamic-tool".to_string(),
        "collabAgentToolCall" => "collab-tool".to_string(),
        "webSearch" => "web-search".to_string(),
        "plan" => "plan".to_string(),
        _ => item_type.to_string(),
    }
}

fn summarize_tool_item(item_type: &str, item: &Value) -> String {
    match item_type {
        "mcpToolCall" => {
            let server = get_string(item, &["server"]).unwrap_or("?");
            let tool = get_string(item, &["tool"]).unwrap_or("?");
            let status = get_string(item, &["status"]).unwrap_or("unknown");
            format!("server={server} tool={tool} status={status}")
        }
        "dynamicToolCall" => {
            let tool = get_string(item, &["tool"]).unwrap_or("?");
            let status = get_string(item, &["status"]).unwrap_or("unknown");
            format!("tool={tool} status={status}")
        }
        "collabAgentToolCall" => {
            let tool = get_string(item, &["tool"]).unwrap_or("?");
            let status = get_string(item, &["status"]).unwrap_or("unknown");
            let prompt = get_string(item, &["prompt"]).unwrap_or("");
            if prompt.is_empty() {
                format!("tool={tool} status={status}")
            } else {
                format!(
                    "tool={tool} status={status} prompt={}",
                    summarize_text(prompt)
                )
            }
        }
        "webSearch" => {
            let query = get_string(item, &["query"]).unwrap_or("");
            let action = get_string(item, &["action", "type"]).unwrap_or("search");
            format!("action={action} query={query}")
        }
        "plan" => get_string(item, &["text"]).unwrap_or("").to_string(),
        _ => summarize_value(item),
    }
}

fn summarize_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Array(values) => values
            .iter()
            .map(summarize_value)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| format!("{key}={}", summarize_value(value)))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use super::Cli;
    use super::build_tool_user_input_response;
    use super::builtin_command_names;
    use super::builtin_help_lines;
    use super::choose_command_approval_decision;
    use super::extract_file_search_paths;
    use super::extract_thread_ids;
    use super::is_builtin_command;
    use super::latest_conversation_history_items;
    use super::normalize_cli;
    use super::params_auto_approval_result;
    use super::parse_feedback_args;
    use super::prompt_accepts_input;
    use super::prompt_is_visible;
    use super::quote_if_needed;
    use super::render_apps_list;
    use super::render_fuzzy_file_search_results;
    use super::render_prompt_status;
    use super::render_rate_limit_lines;
    use super::render_reasoning_item;
    use super::render_slash_completion_candidates;
    use super::render_thread_list;
    use super::seed_resumed_state_from_turns;
    use super::summarize_terminal_interaction;
    use super::summarize_thread_status_for_display;
    use super::try_complete_file_token;
    use super::try_complete_slash_command;
    use crate::editor::LineEditor;
    use crate::input::AppCatalogEntry;
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
        assert!(rendered.contains("switch to plan mode"));
        assert!(rendered.contains(":approvals or /permissions"));
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
