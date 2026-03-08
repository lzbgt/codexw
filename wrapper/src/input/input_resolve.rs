use std::collections::HashMap;
use std::collections::HashSet;

use super::input_decode::collect_prefixed_tokens;
use super::input_decode::is_common_env_var;
use super::input_decode::mention_skill_path;
use super::input_decode::parse_linked_tool_mention;
use super::input_types::AppCatalogEntry;
use super::input_types::PluginCatalogEntry;
use super::input_types::SkillCatalogEntry;

#[derive(Debug, Default, Clone)]
pub(crate) struct ToolMentions {
    pub names: HashSet<String>,
    pub linked_paths: HashMap<String, String>,
}

pub(crate) fn collect_tool_mentions(text: &str) -> ToolMentions {
    let bytes = text.as_bytes();
    let mut names = HashSet::new();
    let mut linked_paths = HashMap::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'['
            && let Some((name, path, end_index)) = parse_linked_tool_mention(text, bytes, index)
        {
            if !is_common_env_var(name) {
                if mention_skill_path(path).is_some() {
                    names.insert(name.to_ascii_lowercase());
                }
                linked_paths
                    .entry(name.to_ascii_lowercase())
                    .or_insert_with(|| path.to_string());
            }
            index = end_index;
            continue;
        }

        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let name_start = index + 1;
        let Some(first) = bytes.get(name_start) else {
            index += 1;
            continue;
        };
        if !super::input_decode::is_mention_name_char(*first) {
            index += 1;
            continue;
        }
        let mut name_end = name_start + 1;
        while let Some(next) = bytes.get(name_end)
            && super::input_decode::is_mention_name_char(*next)
        {
            name_end += 1;
        }
        let name = &text[name_start..name_end];
        if !is_common_env_var(name) {
            names.insert(name.to_ascii_lowercase());
        }
        index = name_end;
    }

    ToolMentions {
        names,
        linked_paths,
    }
}

pub(crate) fn find_skill_mentions(
    mentions: &ToolMentions,
    skills: &[SkillCatalogEntry],
) -> Vec<SkillCatalogEntry> {
    let linked_skill_paths = mentions
        .linked_paths
        .values()
        .filter_map(|path| mention_skill_path(path))
        .collect::<HashSet<_>>();

    let mut selected = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut seen_names = HashSet::new();

    for skill in skills.iter().filter(|skill| skill.enabled) {
        if linked_skill_paths.contains(&skill.path) && seen_paths.insert(skill.path.clone()) {
            seen_names.insert(skill.name.to_ascii_lowercase());
            selected.push(skill.clone());
        }
    }

    for skill in skills.iter().filter(|skill| skill.enabled) {
        let lowered = skill.name.to_ascii_lowercase();
        if mentions.names.contains(&lowered)
            && seen_names.insert(lowered)
            && seen_paths.insert(skill.path.clone())
        {
            selected.push(skill.clone());
        }
    }

    selected
}

pub(crate) fn find_app_mentions(
    mentions: &ToolMentions,
    apps: &[AppCatalogEntry],
    skill_names_lower: &HashSet<String>,
) -> Vec<AppCatalogEntry> {
    let mut explicit_names = HashSet::new();
    let mut selected_ids = HashSet::new();
    for (name, path) in &mentions.linked_paths {
        if let Some(id) = path.strip_prefix("app://")
            && !id.is_empty()
        {
            explicit_names.insert(name.clone());
            selected_ids.insert(id.to_string());
        }
    }

    let mut slug_counts = HashMap::new();
    for app in apps.iter().filter(|app| app.enabled) {
        *slug_counts.entry(app.slug.clone()).or_insert(0usize) += 1;
    }

    for app in apps.iter().filter(|app| app.enabled) {
        let slug = app.slug.to_ascii_lowercase();
        let slug_count = slug_counts.get(&app.slug).copied().unwrap_or(0);
        if mentions.names.contains(&slug)
            && !explicit_names.contains(&slug)
            && slug_count == 1
            && !skill_names_lower.contains(&slug)
        {
            selected_ids.insert(app.id.clone());
        }
    }

    apps.iter()
        .filter(|app| app.enabled && selected_ids.contains(&app.id))
        .cloned()
        .collect()
}

pub(crate) fn find_plugin_mentions(
    text: &str,
    plugins: &[PluginCatalogEntry],
) -> Vec<PluginCatalogEntry> {
    let names = collect_prefixed_tokens(text, '@');
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for plugin in plugins.iter().filter(|plugin| plugin.enabled) {
        let lowered = plugin.name.to_ascii_lowercase();
        if names.contains(&lowered) && seen.insert(plugin.path.clone()) {
            selected.push(plugin.clone());
        }
    }
    selected
}
