use crate::app_input_editor::try_complete_file_token;
use crate::app_input_interrupt::handle_ctrl_c;
use crate::app_input_interrupt::handle_escape;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::prompt_state::prompt_accepts_input;
use crate::prompt_state::prompt_is_visible;
use crate::state::AppState;
use std::process::Command;
use std::process::Stdio;

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

fn spawn_sink_stdin() -> std::process::ChildStdin {
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

#[test]
fn ctrl_c_preserves_draft_while_interrupting_active_turn() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());

    let mut editor = LineEditor::default();
    editor.insert_str("first\nsecond");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result = handle_ctrl_c(&mut state, &mut editor, &mut output, &mut writer).expect("ctrl-c");
    assert!(result.is_none());
    assert_eq!(editor.buffer(), "first\nsecond");
}

#[test]
fn escape_preserves_draft_while_interrupting_active_turn() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    state.turn_running = true;
    state.active_turn_id = Some("turn-1".to_string());

    let mut editor = LineEditor::default();
    editor.insert_str("first\nsecond");
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();

    let result =
        handle_escape(&mut state, &mut editor, &mut output, &mut writer, true).expect("escape");
    assert!(result.is_none());
    assert_eq!(editor.buffer(), "first\nsecond");
}
