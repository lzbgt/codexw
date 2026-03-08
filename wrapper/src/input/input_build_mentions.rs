use std::collections::HashSet;

use serde_json::Value;
use serde_json::json;

use crate::input::input_resolve_catalog::find_app_mentions;
use crate::input::input_resolve_catalog::find_plugin_mentions;
use crate::input::input_resolve_catalog::find_skill_mentions;
use crate::input::input_resolve_tools::collect_tool_mentions;
use crate::input::input_types::AppCatalogEntry;
use crate::input::input_types::PluginCatalogEntry;
use crate::input::input_types::SkillCatalogEntry;

pub(crate) fn push_catalog_mention_items(
    items: &mut Vec<Value>,
    decoded_text: &str,
    apps: &[AppCatalogEntry],
    plugins: &[PluginCatalogEntry],
    skills: &[SkillCatalogEntry],
) {
    let text_mentions = collect_tool_mentions(decoded_text);
    let skill_names_lower = skills
        .iter()
        .filter(|skill| skill.enabled)
        .map(|skill| skill.name.to_ascii_lowercase())
        .collect::<HashSet<_>>();

    for skill in find_skill_mentions(&text_mentions, skills) {
        items.push(json!({
            "type": "skill",
            "name": skill.name,
            "path": skill.path,
        }));
    }

    for app in find_app_mentions(&text_mentions, apps, &skill_names_lower) {
        items.push(json!({
            "type": "mention",
            "name": app.name,
            "path": format!("app://{}", app.id),
        }));
    }

    for plugin in find_plugin_mentions(decoded_text, plugins) {
        items.push(json!({
            "type": "mention",
            "name": plugin.display_name,
            "path": plugin.path,
        }));
    }
}
