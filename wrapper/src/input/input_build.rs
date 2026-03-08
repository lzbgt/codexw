use std::collections::HashSet;

use serde_json::json;

use super::input_decode::decode_linked_mentions;
use super::input_decode::expand_inline_file_mentions;
use super::input_decode::mention_skill_path;
use super::input_resolve::collect_tool_mentions;
use super::input_resolve::find_app_mentions;
use super::input_resolve::find_plugin_mentions;
use super::input_resolve::find_skill_mentions;
use super::input_types::AppCatalogEntry;
use super::input_types::ParsedInput;
use super::input_types::PluginCatalogEntry;
use super::input_types::SkillCatalogEntry;

pub fn build_turn_input(
    text: &str,
    resolved_cwd: &str,
    pending_local_images: &[String],
    pending_remote_images: &[String],
    apps: &[AppCatalogEntry],
    plugins: &[PluginCatalogEntry],
    skills: &[SkillCatalogEntry],
) -> ParsedInput {
    let preprocessed = expand_inline_file_mentions(text, resolved_cwd, plugins);
    let decoded = decode_linked_mentions(&preprocessed);
    let mut items = Vec::new();

    for url in pending_remote_images {
        items.push(json!({
            "type": "image",
            "url": url,
        }));
    }

    for path in pending_local_images {
        items.push(json!({
            "type": "localImage",
            "path": path,
        }));
    }

    if !decoded.text.trim().is_empty() {
        items.push(json!({
            "type": "text",
            "text": decoded.text,
            "text_elements": [],
        }));
    }

    for mention in decoded.mentions {
        if let Some(skill_path) = mention_skill_path(&mention.path) {
            items.push(json!({
                "type": "skill",
                "name": mention.mention,
                "path": skill_path,
            }));
        } else {
            items.push(json!({
                "type": "mention",
                "name": mention.mention,
                "path": mention.path,
            }));
        }
    }

    let text_mentions = collect_tool_mentions(&decoded.text);
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

    for plugin in find_plugin_mentions(&decoded.text, plugins) {
        items.push(json!({
            "type": "mention",
            "name": plugin.display_name,
            "path": plugin.path,
        }));
    }

    ParsedInput {
        display_text: decoded.text,
        items,
    }
}
