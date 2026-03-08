use serde_json::Value;

use crate::input::AppCatalogEntry;
use crate::input::PluginCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::input::build_turn_input;

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
