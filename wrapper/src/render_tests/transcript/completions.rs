use crate::output::render_block_lines_to_ansi;
use crate::transcript_completion_render::abbreviate_long_result_text;
use crate::transcript_completion_render::render_command_completion;
use crate::transcript_completion_render::render_file_change_completion;
use crate::transcript_completion_render::render_local_command_completion;
use crate::transcript_item_summary::humanize_item_type;
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
fn long_file_change_diff_is_not_abbreviated() {
    let long_diff = (1..=81)
        .map(|index| format!("+line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let body = render_file_change_completion(
        &serde_json::json!({
            "changes": [
                {
                    "kind": "update",
                    "path": "docs/FOLLOW_TRADING.md",
                    "diff": long_diff
                }
            ]
        }),
        "completed",
        None,
        false,
    );
    assert!(body.contains("update docs/FOLLOW_TRADING.md"));
    assert!(body.contains("+line 21"));
    assert!(body.contains("+line 81"));
    assert!(!body.contains("\n...\n"));
}

#[test]
fn long_file_change_output_fallback_is_not_abbreviated() {
    let long_output = (1..=81)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let body = render_file_change_completion(
        &serde_json::json!({}),
        "completed",
        Some(&long_output),
        false,
    );
    assert!(body.contains("line 21"));
    assert!(body.contains("line 81"));
    assert!(!body.contains("\n...\n"));
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
fn collab_agent_items_use_humanized_label() {
    assert_eq!(
        humanize_item_type("collabAgentToolCall"),
        "Agent collaboration"
    );
}
