use super::*;

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

    state.startup_resume_picker = true;
    assert!(prompt_is_visible(&state));
    assert!(prompt_accepts_input(&state));

    state.pending_thread_switch = true;
    assert!(!prompt_is_visible(&state));
    assert!(!prompt_accepts_input(&state));

    state.pending_thread_switch = false;
    state.startup_resume_picker = false;
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
fn pasted_multiline_text_is_buffered_without_submit() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();

    crate::app_input_editing::handle_editing_key(
        &InputKey::Paste("first\nsecond".to_string()),
        "/tmp",
        &mut state,
        &mut editor,
        &mut output,
        true,
    )
    .expect("paste");

    assert_eq!(editor.buffer(), "first\nsecond");
    assert_eq!(editor.history.len(), 0);
}

#[test]
fn pasted_text_is_ignored_when_prompt_input_is_disabled() {
    let mut state = AppState::new(true, false);
    state.thread_id = Some("thread-1".to_string());
    let mut editor = LineEditor::default();
    let mut output = Output::default();

    crate::app_input_editing::handle_editing_key(
        &InputKey::Paste("hidden".to_string()),
        "/tmp",
        &mut state,
        &mut editor,
        &mut output,
        false,
    )
    .expect("paste");

    assert!(editor.buffer().is_empty());
}
