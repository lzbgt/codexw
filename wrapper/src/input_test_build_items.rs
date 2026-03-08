use crate::input::build_turn_input;
use crate::input::resolve_file_mention_path;

#[test]
fn build_turn_input_includes_images_text_and_mentions() {
    let parsed = build_turn_input(
        "Open [$figma](app://connector_1)",
        "/tmp",
        &["/tmp/image.png".to_string()],
        &["https://example.com/one.png".to_string()],
        &[],
        &[],
        &[],
    );
    assert_eq!(parsed.display_text, "Open $figma");
    assert_eq!(parsed.items.len(), 4);
    assert_eq!(parsed.items[0]["type"], "image");
    assert_eq!(parsed.items[1]["type"], "localImage");
    assert_eq!(parsed.items[2]["type"], "text");
    assert_eq!(parsed.items[3]["type"], "mention");
}

#[test]
fn build_turn_input_expands_inline_relative_file_mentions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("src").join("main.rs");
    std::fs::create_dir_all(file_path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file_path, "fn main() {}\n").expect("write file");

    let parsed = build_turn_input(
        "inspect @src/main.rs please",
        temp.path().to_str().expect("utf8 cwd"),
        &[],
        &[],
        &[],
        &[],
        &[],
    );

    assert_eq!(parsed.items[0]["type"], "text");
    assert_eq!(parsed.items[0]["text"], "inspect src/main.rs please");
}

#[test]
fn resolve_file_mention_path_quotes_spaces() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("docs").join("hello world.md");
    std::fs::create_dir_all(file_path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file_path, "# hello\n").expect("write file");

    let resolved = resolve_file_mention_path(
        "docs/hello world.md",
        temp.path().to_str().expect("utf8 cwd"),
    );

    assert_eq!(resolved.as_deref(), Some("\"docs/hello world.md\""));
}
