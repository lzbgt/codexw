use crate::editor::LineEditor;
use crate::prompt_file_completions::try_complete_file_token;
use crate::prompt_state::prompt_accepts_input;
use crate::prompt_state::prompt_is_visible;
use crate::state::AppState;

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
