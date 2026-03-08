use crate::output::render_block_lines_to_ansi;
use crate::output::render_line_to_ansi;
use crate::render_prompt::fit_status_line;
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_lines;

#[test]
fn assistant_blocks_render_with_ansi_styling() {
    let rendered = render_block_lines_to_ansi(
        "Assistant",
        "# Heading\n\n- item\n\n```rust\nfn main() {}\n```",
    )
    .join("\n");
    assert!(rendered.contains("\u{1b}["));
    assert!(rendered.contains("Heading"));
    assert!(rendered.contains("fn"));
    assert!(rendered.contains("main"));
}

#[test]
fn diff_blocks_render_colored_lines() {
    let rendered = render_block_lines_to_ansi("Latest diff", "@@ -1 +1 @@\n-old\n+new").join("\n");
    assert!(rendered.contains("old"));
    assert!(rendered.contains("new"));
    assert!(rendered.contains("\u{1b}["));
}

#[test]
fn status_lines_keep_tag_and_content() {
    let rendered = render_line_to_ansi("[ready] all clear");
    assert!(rendered.contains("[ready]"));
    assert!(rendered.contains("all clear"));
}

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
fn prompt_line_previews_multiline_buffer_with_visible_newline_marker() {
    let (rendered, cursor_row, cursor_col) = render_prompt_lines("", "first\nsecond", 12, 10);
    assert!(rendered.join("\n").contains("⏎"));
    assert!(rendered.len() > 1);
    assert!(cursor_row < rendered.len());
    assert!(cursor_col <= 10);
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
