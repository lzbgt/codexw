use crate::output::render_block_lines_to_ansi;
use crate::output::render_line_to_ansi;

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
