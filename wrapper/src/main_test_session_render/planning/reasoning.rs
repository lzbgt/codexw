use crate::transcript_plan_render::render_reasoning_item;
use serde_json::json;

#[test]
fn reasoning_prefers_summary_blocks() {
    let rendered = render_reasoning_item(&json!({
        "summary": ["First block", "Second block"],
        "content": ["raw detail"]
    }));
    assert_eq!(rendered, "First block\n\nSecond block");
}
