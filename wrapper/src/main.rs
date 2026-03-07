mod commands;
mod editor;
mod events;
mod input;
mod output;
mod prompt;
mod render;
mod requests;
mod rpc;
mod session;

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
use session::extract_models;
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
            send_load_models(writer, state, ModelsAction::ShowModels)?;
            Ok(true)
        }
        "mcp" => {
            output.line_stderr("[session] loading MCP server status")?;
            send_load_mcp_servers(writer, state)?;
            Ok(true)
        }
        "clean" => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[thread] background terminal cleanup requires experimental API support; restart without --no-experimental-api",
                )?;
                return Ok(true);
            }
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
        "experimental" => {
            output.line_stderr("[session] loading experimental feature flags")?;
            send_load_experimental_features(writer, state)?;
            Ok(true)
        }
        "personality" => {
            if state.turn_running {
                output
                    .line_stderr("[session] cannot change personality while a turn is running")?;
                return Ok(true);
            }
            let args = parts.collect::<Vec<_>>();
            if args.is_empty() {
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for personality options")?;
                    send_load_models(writer, state, ModelsAction::ShowPersonality)?;
                } else {
                    output.block_stdout("Personality", &render_personality_options(cli, state))?;
                }
                return Ok(true);
            }
            let selector = args.join(" ");
            if state.models.is_empty() {
                output.line_stderr("[session] loading models for personality selection")?;
                send_load_models(writer, state, ModelsAction::SetPersonality(selector))?;
            } else {
                apply_personality_selection(cli, state, &selector, output)?;
            }
            Ok(true)
        }
        "collab" => {
            let args = parts.collect::<Vec<_>>();
            if args.is_empty() {
                send_load_collaboration_modes(writer, state, CollaborationModeAction::ShowList)?;
                return Ok(true);
            }
            if state.turn_running {
                output.line_stderr(
                    "[session] cannot switch collaboration mode while a turn is running",
                )?;
                return Ok(true);
            }
            let selector = args.join(" ");
            send_load_collaboration_modes(
                writer,
                state,
                CollaborationModeAction::SetMode(selector),
            )?;
            Ok(true)
        }
        "plan" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] cannot switch collaboration mode while a turn is running",
                )?;
                return Ok(true);
            }
            send_load_collaboration_modes(writer, state, CollaborationModeAction::TogglePlan)?;
            Ok(true)
        }
        "fast"
        | "agent"
        | "multi-agents"
        | "theme"
        | "rollout"
        | "sandbox-add-read-dir"
        | "setup-default-sandbox"
        | "init" => {
            output.line_stderr(format!(
                "[session] /{command} is recognized, but this inline client does not yet implement the native Codex popup/workflow for it"
            ))?;
            Ok(true)
        }
        "realtime" => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[session] /realtime requires experimental API support; restart without --no-experimental-api",
                )?;
                return Ok(true);
            }
            let args = parts.collect::<Vec<_>>();
            let Some(thread_id) = state.thread_id.clone() else {
                output.line_stderr("[session] start or resume a thread before using /realtime")?;
                return Ok(true);
            };
            if args.is_empty() || matches!(args[0], "status" | "show") {
                output.block_stdout("Realtime", &render_realtime_status(state))?;
                return Ok(true);
            }
            match args[0] {
                "start" => {
                    if state.turn_running {
                        output.line_stderr(
                            "[session] cannot start realtime while a turn is running",
                        )?;
                        return Ok(true);
                    }
                    if state.realtime_active {
                        output.line_stderr(
                            "[session] realtime is already active; use /realtime stop first",
                        )?;
                        output.block_stdout("Realtime", &render_realtime_status(state))?;
                        return Ok(true);
                    }
                    let prompt = if args.len() > 1 {
                        args[1..].join(" ")
                    } else {
                        "Text-only experimental realtime session for this thread.".to_string()
                    };
                    send_thread_realtime_start(writer, state, thread_id, prompt)?;
                }
                "send" | "append" => {
                    if !state.realtime_active {
                        output.line_stderr(
                            "[session] realtime is not active; use /realtime start first",
                        )?;
                        return Ok(true);
                    }
                    if args.len() < 2 {
                        output.line_stderr("[session] usage: /realtime send <text>")?;
                        return Ok(true);
                    }
                    send_thread_realtime_append_text(
                        writer,
                        state,
                        thread_id,
                        args[1..].join(" "),
                    )?;
                }
                "stop" => {
                    if !state.realtime_active {
                        output.line_stderr("[session] realtime is not active")?;
                        return Ok(true);
                    }
                    send_thread_realtime_stop(writer, state, thread_id)?;
                }
                other => {
                    output.line_stderr(format!("[session] unknown realtime action: {other}"))?;
                    output.block_stdout("Realtime", &render_realtime_status(state))?;
                }
            }
            Ok(true)
        }
        "ps" => {
            let action = parts.next();
            if matches!(action, Some("clean")) {
                if cli.no_experimental_api {
                    output.line_stderr(
                        "[thread] /ps clean requires experimental API support; restart without --no-experimental-api",
                    )?;
                    return Ok(true);
                }
                let current_thread_id = thread_id(state)?.to_string();
                output.line_stderr("[thread] cleaning background terminals")?;
                send_clean_background_terminals(writer, state, current_thread_id)?;
                return Ok(true);
            }
            output.line_stderr(
                "[session] app-server does not expose background-terminal listing like the native TUI; use /ps clean to stop all background terminals for this thread",
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

struct FileCompletionResult {
    rendered_candidates: Option<String>,
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

fn render_experimental_features_list(result: &Value) -> String {
    let mut lines = Vec::new();
    let features = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for feature in features {
        let name = feature.get("name").and_then(Value::as_str).unwrap_or("?");
        let stage = feature
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let enabled = feature
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let default_enabled = feature
            .get("defaultEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let display_name = feature
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or(name);
        let description = feature
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let status = if enabled {
            "enabled"
        } else if default_enabled {
            "default-on"
        } else {
            "disabled"
        };

        lines.push(format!("{display_name}  [{stage}] [{status}]"));
        lines.push(format!("  key: {name}"));
        if !description.is_empty() {
            lines.push(format!("  {description}"));
        }
        if let Some(announcement) = feature.get("announcement").and_then(Value::as_str) {
            if !announcement.trim().is_empty() {
                lines.push(format!("  note: {}", summarize_text(announcement)));
            }
        }
        lines.push(String::new());
    }

    if lines.is_empty() {
        lines.push("No experimental features were returned by app-server.".to_string());
    } else {
        lines.pop();
    }

    if result.get("nextCursor").and_then(Value::as_str).is_some() {
        lines.push(String::new());
        lines.push("More feature entries are available from app-server.".to_string());
    }

    lines.join("\n")
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
    let models = extract_models(result);
    if models.is_empty() {
        return "No models returned by app-server.".to_string();
    }
    models
        .iter()
        .take(30)
        .map(|model| {
            let default_marker = if model.is_default { " [default]" } else { "" };
            let personality_marker = if model.supports_personality {
                " [supports personality]"
            } else {
                " [personality unsupported]"
            };
            format!(
                "{} ({}){}{}",
                model.display_name, model.id, default_marker, personality_marker
            )
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
    use super::CollaborationModePreset;
    use super::build_tool_user_input_response;
    use super::builtin_command_names;
    use super::builtin_help_lines;
    use super::choose_command_approval_decision;
    use super::extract_collaboration_mode_presets;
    use super::extract_file_search_paths;
    use super::extract_models;
    use super::extract_thread_ids;
    use super::is_builtin_command;
    use super::latest_conversation_history_items;
    use super::normalize_cli;
    use super::parse_feedback_args;
    use super::prompt_accepts_input;
    use super::prompt_is_visible;
    use super::quote_if_needed;
    use super::render_apps_list;
    use super::render_experimental_features_list;
    use super::render_fuzzy_file_search_results;
    use super::render_models_list;
    use super::render_personality_options;
    use super::render_prompt_status;
    use super::render_rate_limit_lines;
    use super::render_realtime_item;
    use super::render_reasoning_item;
    use super::render_status_snapshot;
    use super::render_thread_list;
    use super::seed_resumed_state_from_turns;
    use super::summarize_terminal_interaction;
    use super::summarize_thread_status_for_display;
    use super::try_complete_file_token;
    use super::try_complete_slash_command;
    use crate::commands::render_slash_completion_candidates;
    use crate::editor::LineEditor;
    use crate::events::params_auto_approval_result;
    use crate::input::AppCatalogEntry;
    use crate::session::render_collaboration_modes;
    use crate::session::summarize_active_collaboration_mode;
    use crate::session::summarize_active_personality;
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
