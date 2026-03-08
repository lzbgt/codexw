use serde_json::Value;

use crate::input::AppCatalogEntry;
use crate::input::PluginCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::input::build_turn_input;

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
fn build_turn_input_resolves_raw_catalog_mentions() {
    let parsed = build_turn_input(
        "$demo-app check this with @sample and $deploy",
        "/tmp",
        &[],
        &[],
        &[AppCatalogEntry {
            id: "connector_1".to_string(),
            name: "Demo App".to_string(),
            slug: "demo-app".to_string(),
            enabled: true,
        }],
        &[PluginCatalogEntry {
            name: "sample".to_string(),
            display_name: "Sample Plugin".to_string(),
            path: "plugin://sample@test".to_string(),
            enabled: true,
        }],
        &[SkillCatalogEntry {
            name: "deploy".to_string(),
            path: "/tmp/deploy/SKILL.md".to_string(),
            enabled: true,
        }],
    );
    assert_eq!(parsed.items.len(), 4);
    assert_eq!(parsed.items[0]["type"], "text");
    let paths = parsed
        .items
        .iter()
        .skip(1)
        .filter_map(|item| item.get("path").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(paths.contains(&"app://connector_1"));
    assert!(paths.contains(&"/tmp/deploy/SKILL.md"));
    assert!(paths.contains(&"plugin://sample@test"));
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
