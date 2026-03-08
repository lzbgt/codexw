use crate::editor::EditorEvent;
use crate::editor::LineEditor;

#[test]
fn supports_cursor_movement_and_delete() {
    let mut editor = LineEditor::default();
    for ch in "hello".chars() {
        editor.insert_char(ch);
    }
    editor.move_left();
    editor.move_left();
    editor.delete();
    assert_eq!(editor.buffer(), "helo");
    assert_eq!(editor.cursor_chars(), 3);
}

#[test]
fn backspace_removes_previous_character() {
    let mut editor = LineEditor::default();
    for ch in "abc".chars() {
        editor.insert_char(ch);
    }
    editor.backspace();
    assert_eq!(editor.buffer(), "ab");
    assert_eq!(editor.cursor_chars(), 2);
}

#[test]
fn backspace_removes_previous_newline_boundary() {
    let mut editor = LineEditor::default();
    editor.insert_str("ab\ncd");
    editor.move_left();
    editor.move_left();
    editor.backspace();
    assert_eq!(editor.buffer(), "abcd");
    assert_eq!(editor.cursor_chars(), 2);
}

#[test]
fn delete_removes_next_newline_boundary() {
    let mut editor = LineEditor::default();
    editor.insert_str("ab\ncd");
    editor.move_left();
    editor.move_left();
    editor.move_left();
    editor.delete();
    assert_eq!(editor.buffer(), "abcd");
    assert_eq!(editor.cursor_chars(), 2);
}

#[test]
fn backspace_removes_previous_emoji_grapheme() {
    let mut editor = LineEditor::default();
    editor.insert_str("a🙂b");
    editor.move_left();
    editor.backspace();
    assert_eq!(editor.buffer(), "ab");
    assert_eq!(editor.cursor_chars(), 1);
}

#[test]
fn backspace_removes_previous_combining_grapheme() {
    let mut editor = LineEditor::default();
    editor.insert_str("e\u{301}x");
    editor.move_left();
    editor.backspace();
    assert_eq!(editor.buffer(), "x");
    assert_eq!(editor.cursor_chars(), 0);
}

#[test]
fn history_navigation_restores_draft() {
    let mut editor = LineEditor::default();
    for ch in "first".chars() {
        editor.insert_char(ch);
    }
    assert_eq!(editor.submit(), EditorEvent::Submit("first".to_string()));
    for ch in "second".chars() {
        editor.insert_char(ch);
    }
    assert_eq!(editor.submit(), EditorEvent::Submit("second".to_string()));
    for ch in "dra".chars() {
        editor.insert_char(ch);
    }
    editor.history_prev();
    assert_eq!(editor.buffer(), "second");
    editor.history_prev();
    assert_eq!(editor.buffer(), "first");
    editor.history_next();
    assert_eq!(editor.buffer(), "second");
    editor.history_next();
    assert_eq!(editor.buffer(), "dra");
}

#[test]
fn ctrl_u_clears_to_start_of_line() {
    let mut editor = LineEditor::default();
    for ch in "hello world".chars() {
        editor.insert_char(ch);
    }
    editor.move_left();
    editor.move_left();
    editor.clear_to_start();
    assert_eq!(editor.buffer(), "ld");
    assert_eq!(editor.cursor_chars(), 0);
}

#[test]
fn home_and_end_stay_within_current_multiline_segment() {
    let mut editor = LineEditor::default();
    editor.insert_str("alpha\nbeta\ngamma");
    editor.move_left();
    editor.move_left();
    editor.move_left();
    editor.move_left();
    editor.move_left();
    editor.move_left();
    editor.move_left();

    editor.move_home();
    assert_eq!(editor.cursor_chars(), "alpha\n".chars().count());

    editor.move_end();
    assert_eq!(editor.cursor_chars(), "alpha\nbeta".chars().count());
}

#[test]
fn up_and_down_move_within_multiline_draft_instead_of_history() {
    let mut editor = LineEditor::default();
    editor.insert_str("alpha\nbeta\ngamma");
    editor.move_home();
    assert_eq!(editor.cursor_chars(), "alpha\nbeta\n".chars().count());
    editor.move_up();
    assert_eq!(editor.cursor_chars(), "alpha\n".chars().count());

    editor.move_up();
    assert_eq!(editor.cursor_chars(), 0);

    editor.move_down();
    assert_eq!(editor.cursor_chars(), "alpha\n".chars().count());

    editor.move_end();
    editor.move_down();
    assert_eq!(editor.cursor_chars(), "alpha\nbeta\ngamm".chars().count());
}

#[test]
fn ctrl_u_clears_only_current_multiline_segment_prefix() {
    let mut editor = LineEditor::default();
    editor.insert_str("alpha\nbeta");
    editor.move_left();
    editor.move_left();
    editor.clear_to_start();
    assert_eq!(editor.buffer(), "alpha\nta");
    assert_eq!(editor.cursor_chars(), "alpha\n".chars().count());
}

#[test]
fn ctrl_w_deletes_previous_word() {
    let mut editor = LineEditor::default();
    for ch in "hello brave world".chars() {
        editor.insert_char(ch);
    }
    editor.delete_prev_word();
    assert_eq!(editor.buffer(), "hello brave ");
    assert_eq!(editor.cursor_chars(), "hello brave ".chars().count());
}

#[test]
fn insert_newline_preserves_multiline_submit() {
    let mut editor = LineEditor::default();
    editor.insert_str("first");
    editor.insert_newline();
    editor.insert_str("second");
    assert_eq!(editor.buffer(), "first\nsecond");
    assert_eq!(
        editor.submit(),
        EditorEvent::Submit("first\nsecond".to_string())
    );
}
