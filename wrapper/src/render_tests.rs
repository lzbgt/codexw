use crate::render_blocks::render_block_lines_to_ansi;
use crate::render_blocks::render_line_to_ansi;
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_line;

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
fn prompt_line_is_elided_to_stay_single_row() {
    let (rendered, cursor_col) = render_prompt_line(
        "",
        "continue working on the highest leverage task in this repository",
        62,
        40,
    );
    assert!(rendered.contains(">"));
    assert!(rendered.contains("..."));
    assert!(cursor_col <= 40);
}

#[test]
fn prompt_line_previews_multiline_buffer_in_single_row() {
    let (rendered, cursor_col) = render_prompt_line("", "first\nsecond", 12, 40);
    assert!(rendered.contains("↩"));
    assert!(cursor_col <= 40);
}

#[test]
fn prompt_line_accounts_for_wide_graphemes_in_cursor_position() {
    let (_rendered, cursor_col) = render_prompt_line("", "a🙂中", 3, 20);
    assert_eq!(cursor_col, 7);
}

#[test]
fn prompt_line_handles_combining_graphemes_without_overadvancing_cursor() {
    let (_rendered, cursor_col) = render_prompt_line("", "e\u{301}x", 1, 20);
    assert_eq!(cursor_col, 3);
}
