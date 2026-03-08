use crate::commands_completion_apply::try_complete_slash_command;
use crate::dispatch_commands::is_builtin_command;
use crate::editor::LineEditor;

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
