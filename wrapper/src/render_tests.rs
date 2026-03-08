use crate::output::render_block_lines_to_ansi;
use crate::output::render_line_to_ansi;
use crate::render_prompt::fit_status_line;
use crate::render_prompt::render_committed_prompt;
use crate::render_prompt::render_prompt_lines;
use crate::transcript_completion_render::abbreviate_long_result_text;
use crate::transcript_completion_render::render_command_completion;
use crate::transcript_completion_render::render_file_change_completion;
use crate::transcript_completion_render::render_local_command_completion;
use crate::transcript_item_summary::summarize_tool_item;

fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

#[test]
fn assistant_blocks_render_with_ansi_styling() {
    let rendered = render_block_lines_to_ansi(
        "Assistant",
        "# Heading\n\n- item\n\n```rust\nfn main() {}\n```",
    )
    .join("\n");
    let visible = strip_ansi(&rendered);
    assert!(rendered.contains("\u{1b}["));
    assert!(rendered.contains("Heading"));
    assert!(rendered.contains("fn"));
    assert!(rendered.contains("main"));
    assert!(!visible.contains("Assistant"));
    assert!(visible.contains("• # Heading"));
}

#[test]
fn resumed_user_blocks_render_without_user_caption() {
    let rendered = render_block_lines_to_ansi("User", "resume this session").join("\n");
    let visible = strip_ansi(&rendered);
    assert!(!visible.contains("User"));
    assert!(visible.contains("› resume this session"));
}

#[test]
fn diff_blocks_render_colored_lines() {
    let rendered = render_block_lines_to_ansi("Latest diff", "@@ -1 +1 @@\n-old\n+new").join("\n");
    assert!(rendered.contains("old"));
    assert!(rendered.contains("new"));
    assert!(rendered.contains("\u{1b}["));
}

#[test]
fn successful_command_completion_hides_status_and_exit_metadata() {
    let body = render_command_completion(
        "/bin/zsh -lc \"sed -n '96,116p' README.md\"",
        "completed",
        "0",
        None,
        false,
    );
    assert!(body.contains("$ /bin/zsh -lc \"sed -n '96,116p' README.md\""));
    assert!(!body.contains("\nstatus  completed\nexit    0"));
    assert!(!body.contains("[status]"));
    assert!(!body.contains("[exit]"));

    let rendered = render_block_lines_to_ansi("Command complete", &body).join("\n");
    let visible = strip_ansi(&rendered);
    assert!(visible.contains("$ /bin/zsh -lc \"sed -n '96,116p' README.md\""));
    assert!(!visible.contains("status"));
    assert!(!visible.contains("completed"));
    assert!(!visible.contains("exit"));
    assert!(!visible.contains("Command complete"));
}

#[test]
fn failed_command_completion_keeps_status_and_exit_metadata() {
    let body = render_command_completion("/bin/zsh -lc false", "failed", "1", None, false);
    assert!(body.contains("\nstatus  failed\nexit    1"));
}

#[test]
fn successful_local_command_hides_exit_and_stdout_label_when_unneeded() {
    let body = render_local_command_completion("sed -n '1,5p' file", "0", "hello\n", "", false);
    assert_eq!(body, "$ sed -n '1,5p' file\n\nhello");
}

#[test]
fn failed_local_command_keeps_exit_and_stream_labels_when_needed() {
    let body = render_local_command_completion("false", "1", "out\n", "err\n", false);
    assert!(body.contains("\nexit    1"));
    assert!(body.contains("[stdout]\nout"));
    assert!(body.contains("[stderr]\nerr"));
}

#[test]
fn completed_file_change_hides_redundant_status_and_summary_when_structured() {
    let body = render_file_change_completion(
        &serde_json::json!({
            "changes": [
                {"kind": "update", "path": "docs/FOLLOW_TRADING.md"}
            ]
        }),
        "completed",
        None,
        false,
    );
    assert_eq!(body, "update docs/FOLLOW_TRADING.md");
}

#[test]
fn failed_file_change_keeps_status_context() {
    let body = render_file_change_completion(
        &serde_json::json!({
            "changes": [
                {"kind": "update", "path": "docs/FOLLOW_TRADING.md"}
            ]
        }),
        "failed",
        None,
        false,
    );
    assert!(body.contains("status  failed"));
    assert!(body.contains("update docs/FOLLOW_TRADING.md"));
}

#[test]
fn long_result_text_is_abbreviated_to_head_and_tail() {
    let body = (1..=81)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let abbreviated = abbreviate_long_result_text(&body, false);
    assert!(abbreviated.contains("line 1"));
    assert!(abbreviated.contains("line 20"));
    assert!(abbreviated.contains("\n...\n"));
    assert!(abbreviated.contains("line 62"));
    assert!(abbreviated.contains("line 81"));
    assert!(!abbreviated.contains("line 21"));
}

#[test]
fn long_command_output_is_abbreviated() {
    let long_output = (1..=81)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let body =
        render_command_completion("cat long.log", "completed", "0", Some(&long_output), false);
    assert!(body.contains("\n...\n"));
    assert!(!body.contains("line 21"));
    assert!(body.contains("line 62"));
}

#[test]
fn long_tool_text_result_is_abbreviated() {
    let long_text = (1..=81)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let rendered = summarize_tool_item(
        "dynamicToolCall",
        &serde_json::json!({
            "tool": "workspace_read_file",
            "contentItems": [
                {"text": long_text}
            ]
        }),
        false,
    );
    assert!(rendered.contains("workspace_read_file"));
    assert!(rendered.contains("\n...\n"));
    assert!(!rendered.contains("line 21"));
    assert!(rendered.contains("line 62"));
}

#[test]
fn verbose_mode_preserves_full_long_result_text() {
    let body = (1..=81)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let abbreviated = abbreviate_long_result_text(&body, true);
    assert!(abbreviated.contains("line 21"));
    assert!(!abbreviated.contains("\n...\n"));
}

#[test]
fn updated_plan_blocks_use_checkbox_style() {
    let rendered = render_block_lines_to_ansi(
        "Updated Plan",
        "Adapting plan\n✔ Explore codebase\n□ Implement feature\n◦ Write tests",
    )
    .join("\n");
    let visible = strip_ansi(&rendered);
    assert!(visible.contains("Updated Plan"));
    assert!(visible.contains("• Updated Plan"));
    assert!(visible.contains("✔ Explore codebase"));
    assert!(visible.contains("□ Implement feature"));
    assert!(visible.contains("◦ Write tests"));
    assert!(rendered.contains("\u{1b}["));
}

#[test]
fn proposed_plan_blocks_render_markdown_body() {
    let rendered =
        render_block_lines_to_ansi("Proposed Plan", "## Plan\n\n1. Inspect\n2. Patch").join("\n");
    let visible = strip_ansi(&rendered);
    assert!(visible.contains("Proposed Plan"));
    assert!(visible.contains("• Proposed Plan"));
    assert!(visible.contains("## Plan"));
    assert!(visible.contains("1. Inspect"));
    assert!(visible.contains("2. Patch"));
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
