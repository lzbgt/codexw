use super::*;
use crate::output::Output;
use crate::rpc::RequestId;
use crate::rpc::RpcRequest;
use crate::state::AppState;
use serde_json::json;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;

fn spawn_recording_stdin() -> (TempDir, Child, std::process::ChildStdin, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("requests.jsonl");
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("cat > \"$1\"")
        .arg("sh")
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn recorder");
    let stdin = child.stdin.take().expect("stdin");
    (temp, child, stdin, path)
}

fn read_recorded_requests(
    child: &mut Child,
    writer: std::process::ChildStdin,
    path: &std::path::Path,
) -> Vec<serde_json::Value> {
    drop(writer);
    child.wait().expect("wait recorder");
    let contents = std::fs::read_to_string(path).expect("read requests");
    contents
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("parse request"))
        .collect()
}

fn test_request(method: &str, tool: &str, arguments: serde_json::Value) -> RpcRequest {
    RpcRequest {
        id: RequestId::Integer(7),
        method: method.to_string(),
        params: json!({
            "tool": tool,
            "threadId": "thread-1",
            "callId": "call-1",
            "arguments": arguments,
        }),
    }
}

#[test]
fn tool_requests_are_rejected_for_model_sessions() {
    let mut state = AppState::new(true, false);
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();
    let (tx, rx) = mpsc::channel();
    let request = test_request("item/tool/call", "lookup_ticket", json!({"id": "ABC-123"}));

    let handled = handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
        .expect("handle tool request");

    assert!(handled);
    assert!(state.active_async_tool_requests.is_empty());
    assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());

    let responses = read_recorded_requests(&mut child, writer, &path);
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], json!(7));
    assert_eq!(responses[0]["result"]["success"], false);
    assert_eq!(
        responses[0]["result"]["failure_kind"],
        "tool_request_rejected"
    );
    let text = responses[0]["result"]["contentItems"][0]["text"]
        .as_str()
        .expect("tool rejection text");
    assert!(text.contains("lookup_ticket"));
    assert!(text.contains("rejected"));
}

#[test]
fn non_tool_requests_are_ignored() {
    let mut state = AppState::new(true, false);
    let mut output = Output::default();
    let (_temp, mut child, mut writer, path) = spawn_recording_stdin();
    let (tx, rx) = mpsc::channel();
    let request = test_request("thread/unknown", "lookup_ticket", json!({"id": "ABC-123"}));

    let handled = handle_tool_request(&request, "/tmp", &mut state, &mut output, &mut writer, &tx)
        .expect("handle tool request");

    assert!(!handled);
    assert!(state.active_async_tool_requests.is_empty());
    assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());

    let responses = read_recorded_requests(&mut child, writer, &path);
    assert!(responses.is_empty());
}
