use std::collections::HashMap;

use base64::Engine;
use serde_json::json;

use super::buffer_item_delta;
use super::buffer_process_delta;
use super::canonicalize_or_keep;
use super::get_string;
use super::summarize_text;
use crate::state::ProcessOutputBuffer;

#[test]
fn get_string_reads_nested_paths() {
    let value = json!({
        "turn": {
            "status": "completed"
        }
    });
    assert_eq!(get_string(&value, &["turn", "status"]), Some("completed"));
    assert_eq!(get_string(&value, &["turn", "missing"]), None);
}

#[test]
fn summarize_text_flattens_whitespace_and_truncates() {
    assert_eq!(summarize_text("a  b\nc"), "a b c");
    let long = "x".repeat(200);
    let summarized = summarize_text(&long);
    assert!(summarized.ends_with("..."));
    assert!(summarized.chars().count() <= 120);
}

#[test]
fn buffer_item_delta_appends_by_item_id() {
    let mut buffers = HashMap::new();
    buffer_item_delta(&mut buffers, &json!({"itemId": "item-1", "delta": "hello"}));
    buffer_item_delta(
        &mut buffers,
        &json!({"itemId": "item-1", "delta": " world"}),
    );
    assert_eq!(
        buffers.get("item-1").map(String::as_str),
        Some("hello world")
    );
}

#[test]
fn buffer_process_delta_routes_stdout_and_stderr() {
    let mut buffers: HashMap<String, ProcessOutputBuffer> = HashMap::new();
    let stdout = base64::engine::general_purpose::STANDARD.encode("out");
    let stderr = base64::engine::general_purpose::STANDARD.encode("err");
    buffer_process_delta(
        &mut buffers,
        &json!({"processId": "proc-1", "deltaBase64": stdout, "stream": "stdout"}),
    );
    buffer_process_delta(
        &mut buffers,
        &json!({"processId": "proc-1", "deltaBase64": stderr, "stream": "stderr"}),
    );
    let buffer = buffers.get("proc-1").expect("process buffer");
    assert_eq!(buffer.stdout, "out");
    assert_eq!(buffer.stderr, "err");
}

#[test]
fn canonicalize_or_keep_returns_original_for_missing_path() {
    let missing = "/definitely/missing/codexw-state-helper-test-path";
    assert_eq!(canonicalize_or_keep(missing), missing);
}
