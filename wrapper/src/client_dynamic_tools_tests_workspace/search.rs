use super::BackgroundShellManager;
use super::execute_dynamic_tool_call;
use super::json;

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
