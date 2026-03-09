use super::execute_dynamic_tool_call;
use crate::background_shells::BackgroundShellManager;
use serde_json::json;

#[test]
fn workspace_list_dir_returns_sorted_entries() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(workspace.path().join("src")).expect("mkdir");
    std::fs::write(workspace.path().join("a.txt"), "alpha").expect("write");
    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn demo() {}\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_list_dir",
            "arguments": {"path": ".", "limit": 10}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("Directory: ."));
    assert!(text.contains("file  5 bytes"));
    assert!(text.contains("a.txt"));
    assert!(text.contains("dir   -"));
    assert!(text.contains("src"));
}

#[test]
fn workspace_stat_path_reports_type_and_size() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("hello.txt"), "alpha").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_stat_path",
            "arguments": {"path": "hello.txt"}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("Path: hello.txt"));
    assert!(text.contains("Type: file"));
    assert!(text.contains("Size: 5 bytes"));
}

#[test]
fn workspace_read_file_returns_line_numbered_content() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("hello.txt"), "alpha\nbeta\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_read_file",
            "arguments": {"path": "hello.txt", "startLine": 2}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("File: hello.txt"));
    assert!(text.contains("   2 | beta"));
}

#[test]
fn workspace_search_text_returns_matching_lines() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("src.txt"), "alpha\nneedle here\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_search_text",
            "arguments": {"query": "needle"}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("Text matches for `needle`:"));
    assert!(text.contains("src.txt:2: needle here"));
}

#[test]
fn workspace_find_files_returns_relative_paths() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(workspace.path().join("src")).expect("mkdir");
    std::fs::write(workspace.path().join("src/lib.rs"), "pub fn demo() {}\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_find_files",
            "arguments": {"query": "lib"}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("File matches for `lib`:"));
    assert!(text.contains("src/lib.rs"));
}

#[test]
fn workspace_read_file_rejects_escape_outside_workspace() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::NamedTempFile::new().expect("tempfile");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_read_file",
            "arguments": {"path": outside.path()}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], false);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("outside the current workspace"));
}
