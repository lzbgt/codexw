use std::sync::mpsc;

use anyhow::Context;
use anyhow::Result;

use crate::Cli;
use crate::app_input_controls::handle_control_key;
use crate::app_input_editing::handle_editing_key;
use crate::dispatch_command_utils::join_prompt;
use crate::editor::LineEditor;
use crate::events::process_server_line;
use crate::output::Output;
use crate::prompt_state::prompt_accepts_input;
use crate::prompt_state::update_prompt;
use crate::requests::send_initialize;
use crate::runtime_event_sources::AppEvent;
use crate::runtime_event_sources::RawModeGuard;
use crate::runtime_event_sources::start_stdin_thread;
use crate::runtime_event_sources::start_stdout_thread;
use crate::runtime_event_sources::start_tick_thread;
use crate::runtime_keys::InputKey;
use crate::runtime_process::StartMode;
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
        key,
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
    handle_editing_key(key, resolved_cwd, state, editor, output, accepts_input)?;
    Ok(true)
}
