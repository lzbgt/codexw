use std::sync::mpsc;

use anyhow::Context;
use anyhow::Result;

use crate::Cli;
use crate::app::resume::emit_resume_exit_hint;
use crate::app_input_controls::handle_control_key;
use crate::app_input_editing::handle_editing_key;
use crate::config_persistence::load_persisted_theme;
use crate::dispatch_command_utils::join_prompt;
use crate::editor::LineEditor;
use crate::events::process_server_line;
use crate::local_api::new_process_session_id;
use crate::local_api::new_shared_snapshot;
use crate::local_api::start_local_api;
use crate::local_api::sync_shared_snapshot;
use crate::output::Output;
use crate::prompt_state::prompt_accepts_input;
use crate::prompt_state::update_prompt;
use crate::render_markdown_code::set_theme;
use crate::requests::send_initialize;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_event_sources::RawModeGuard;
use crate::runtime_event_sources::start_stdin_thread;
use crate::runtime_event_sources::start_stdout_thread;
use crate::runtime_event_sources::start_tick_thread;
use crate::runtime_keys::InputKey;
use crate::runtime_process::build_start_mode;
use crate::runtime_process::effective_cwd;
use crate::runtime_process::shutdown_child;
use crate::runtime_process::spawn_server;
use crate::state::AppState;

pub(crate) fn run(cli: Cli) -> Result<()> {
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
    let local_api_snapshot = new_shared_snapshot(new_process_session_id(), resolved_cwd.clone());
    sync_shared_snapshot(&local_api_snapshot, &state);
    let local_api_handle = start_local_api(&cli, local_api_snapshot.clone())?;
    if let Some(handle) = local_api_handle.as_ref() {
        output.line_stderr(format!(
            "[session] local API listening on http://{}",
            handle.bind_addr()
        ))?;
    }

    match load_persisted_theme() {
        Ok(Some(theme_name)) => set_theme(&theme_name),
        Ok(None) => {}
        Err(err) => output.line_stderr(format!("[session] failed to load saved theme: {err:#}"))?,
    }

    output.line_stderr("[session] connecting to codex app-server")?;
    send_initialize(&mut writer, &mut state, &cli, !cli.no_experimental_api)?;

    let mut start_after_initialize = Some(build_start_mode(&cli, initial_prompt));

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
            Ok(AppEvent::InputKey(key)) => {
                if !handle_input_key(
                    key,
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
        sync_shared_snapshot(&local_api_snapshot, &state);
    }

    emit_resume_exit_hint(&mut output, &state, &resolved_cwd)?;
    if let Some(handle) = local_api_handle {
        handle.shutdown()?;
    }
    shutdown_child(writer, child)?;
    Ok(())
}

fn handle_input_key(
    key: InputKey,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<bool> {
    let accepts_input = prompt_accepts_input(state);
    if let Some(continue_running) = handle_control_key(
        &key,
        cli,
        resolved_cwd,
        state,
        editor,
        output,
        writer,
        accepts_input,
    )? {
        return Ok(continue_running);
    }
    handle_editing_key(&key, resolved_cwd, state, editor, output, accepts_input)?;
    Ok(true)
}
