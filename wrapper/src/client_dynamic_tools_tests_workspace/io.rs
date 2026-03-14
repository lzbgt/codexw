use super::BackgroundShellManager;
use super::execute_dynamic_tool_call;
use super::json;

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
fn workspace_list_dir_limits_output_without_losing_sorted_order() {
    let workspace = tempfile::tempdir().expect("tempdir");
    for name in ["z-last.txt", "m-middle.txt", "a-first.txt", "b-second.txt"] {
        std::fs::write(workspace.path().join(name), name).expect("write");
    }

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_list_dir",
            "arguments": {"path": ".", "limit": 2}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    let lines = text.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "Directory: .");
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[3], "... more entries omitted");
}

#[test]
fn workspace_list_dir_sorts_returned_subset_for_small_directories() {
    let workspace = tempfile::tempdir().expect("tempdir");
    for name in ["z-last.txt", "a-first.txt"] {
        std::fs::write(workspace.path().join(name), name).expect("write");
    }

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_list_dir",
            "arguments": {"path": ".", "limit": 5}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    let lines = text.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "Directory: .");
    assert!(lines[1].contains("a-first.txt"));
    assert!(lines[2].contains("z-last.txt"));
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
