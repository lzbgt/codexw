use crate::render_prompt::fit_status_line;
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_lines;

#[test]
fn committed_prompt_preserves_multiline_structure() {
    let rendered = render_committed_prompt("first\nsecond");
    assert!(rendered.contains("first"));
    assert!(rendered.contains("second"));
}

#[test]
fn prompt_line_wraps_when_buffer_overflows_terminal_width() {
    let (rendered, cursor_row, cursor_col) = render_prompt_lines(
        "",
        "continue working on the highest leverage task in this repository",
        62,
        24,
    );
    assert!(rendered.len() > 1);
    assert!(rendered[0].contains(">"));
    assert!(cursor_row < rendered.len());
    assert!(cursor_col <= 24);
}

#[test]
fn prompt_line_renders_multiline_buffer_as_visual_lines() {
    let (rendered, cursor_row, cursor_col) = render_prompt_lines("", "first\nsecond", 12, 10);
    assert!(rendered.len() > 1);
    assert_eq!(cursor_row, 1);
    assert!(cursor_col <= 10);
    assert!(rendered[0].contains("first"));
    assert!(rendered[1].contains("second"));
}

#[test]
fn status_line_is_elided_to_stay_single_row() {
    let rendered = fit_status_line(
        "⠏ updating /Users/zongbaolu/work/ploymarket/.env.example, /Users/zongbaolu/work/ploymarket/docs/MILESTONE_B.md, /Users/zongbaolu/work/ploymarket/docs",
        60,
    );
    assert!(rendered.contains("⠏"));
    assert!(rendered.ends_with("..."));
    assert!(unicode_width::UnicodeWidthStr::width(rendered.as_str()) <= 60);
}

#[test]
fn prompt_line_accounts_for_wide_graphemes_in_cursor_position() {
    let (_rendered, _cursor_row, cursor_col) = render_prompt_lines("", "a🙂中", 3, 20);
    assert_eq!(cursor_col, 7);
}

#[test]
fn prompt_line_handles_combining_graphemes_without_overadvancing_cursor() {
    let (_rendered, _cursor_row, cursor_col) = render_prompt_lines("", "e\u{301}x", 1, 20);
    assert_eq!(cursor_col, 3);
}
