#[path = "session/attachment.rs"]
mod attachment;
#[path = "session/lifecycle.rs"]
mod lifecycle;
#[path = "session/read.rs"]
mod read;

use serde_json::Value;

fn assert_json_path_eq(body: &Value, path: &str, expected: &str, context: &str) {
    let mut current = body;
    for segment in path.split('.') {
        current = current
            .get(segment)
            .unwrap_or_else(|| panic!("missing path segment {segment} for {context}: {body}"));
    }
    assert_eq!(
        current,
        &Value::String(expected.to_string()),
        "unexpected value for {path} in {context}"
    );
}
