use std::sync::mpsc;

use anyhow::Context;
use anyhow::Result;

use crate::Cli;
use crate::editor::EditorEvent;
use crate::editor::LineEditor;
use crate::events::process_server_line;
use crate::interaction::handle_tab_completion;
use crate::interaction::handle_user_input;
use crate::interaction::join_prompt;
use crate::interaction::prompt_accepts_input;
use crate::interaction::update_prompt;
use crate::output::Output;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_initialize;
use crate::requests::send_turn_interrupt;
use crate::runtime::AppEvent;
use crate::runtime::InputKey;
use crate::runtime::RawModeGuard;
use crate::runtime::StartMode;
use crate::runtime::effective_cwd;
use crate::runtime::shutdown_child;
use crate::runtime::spawn_server;
use crate::runtime::start_stdin_thread;
use crate::runtime::start_stdout_thread;
use crate::runtime::start_tick_thread;
use crate::state::AppState;
use crate::state::thread_id;

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
    match key {
        InputKey::Char(ch) => {
            if prompt_accepts_input(state) {
                editor.insert_char(ch);
            }
        }
        InputKey::Esc => {
            if state.turn_running {
                if let Some(turn_id) = state.active_turn_id.clone() {
                    let current_thread_id = thread_id(state)?.to_string();
                    output.line_stderr("[interrupt] interrupting active turn")?;
                    send_turn_interrupt(writer, state, current_thread_id, turn_id)?;
                } else {
                    output.line_stderr("[session] no active turn id; exiting")?;
                    return Ok(false);
                }
            } else if let Some(process_id) = state.active_exec_process_id.clone() {
                output.line_stderr("[interrupt] terminating active local command")?;
                send_command_exec_terminate(writer, state, process_id)?;
            } else if prompt_accepts_input(state) {
                editor.clear();
            }
        }
        InputKey::Backspace => {
            if prompt_accepts_input(state) {
                editor.backspace();
            }
        }
        InputKey::Delete => {
            if prompt_accepts_input(state) {
                editor.delete();
            }
        }
        InputKey::Left => {
            if prompt_accepts_input(state) {
                editor.move_left();
            }
        }
        InputKey::Right => {
            if prompt_accepts_input(state) {
                editor.move_right();
            }
        }
        InputKey::Home => {
            if prompt_accepts_input(state) {
                editor.move_home();
            }
        }
        InputKey::End => {
            if prompt_accepts_input(state) {
                editor.move_end();
            }
        }
        InputKey::Up => {
            if prompt_accepts_input(state) {
                editor.history_prev();
            }
        }
        InputKey::Down => {
            if prompt_accepts_input(state) {
                editor.history_next();
            }
        }
        InputKey::Tab => {
            if prompt_accepts_input(state) {
                handle_tab_completion(editor, state, resolved_cwd, output)?;
            }
        }
        InputKey::CtrlA => {
            if prompt_accepts_input(state) {
                editor.move_home();
            }
        }
        InputKey::CtrlE => {
            if prompt_accepts_input(state) {
                editor.move_end();
            }
        }
        InputKey::CtrlU => {
            if prompt_accepts_input(state) {
                editor.clear_to_start();
            }
        }
        InputKey::CtrlW => {
            if prompt_accepts_input(state) {
                editor.delete_prev_word();
            }
        }
        InputKey::CtrlC => {
            if state.turn_running {
                editor.clear();
                if let Some(turn_id) = state.active_turn_id.clone() {
                    let current_thread_id = thread_id(state)?.to_string();
                    output.line_stderr("[interrupt] interrupting active turn")?;
                    send_turn_interrupt(writer, state, current_thread_id, turn_id)?;
                } else {
                    output.line_stderr("[session] no active turn id; exiting")?;
                    return Ok(false);
                }
            } else if let Some(process_id) = state.active_exec_process_id.clone() {
                editor.clear();
                output.line_stderr("[interrupt] terminating active local command")?;
                send_command_exec_terminate(writer, state, process_id)?;
            } else if matches!(editor.ctrl_c(), EditorEvent::CtrlC) {
                output.line_stderr("[session] exiting on Ctrl-C")?;
                return Ok(false);
            }
        }
        InputKey::Enter => match editor.submit() {
            EditorEvent::Submit(line) => {
                output.commit_prompt(&line)?;
                if !handle_user_input(line, cli, resolved_cwd, state, editor, output, writer)? {
                    return Ok(false);
                }
            }
            EditorEvent::CtrlC | EditorEvent::Noop => {}
        },
        InputKey::CtrlJ => {
            if prompt_accepts_input(state) {
                editor.insert_newline();
            }
        }
    }
    Ok(true)
}
