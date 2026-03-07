use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::Value;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedMention {
    pub mention: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInput {
    pub display_text: String,
    pub items: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCatalogEntry {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCatalogEntry {
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCatalogEntry {
    pub name: String,
    pub path: String,
    pub enabled: bool,
}

pub fn build_turn_input(
    text: &str,
    pending_local_images: &[String],
    pending_remote_images: &[String],
    apps: &[AppCatalogEntry],
    plugins: &[PluginCatalogEntry],
    skills: &[SkillCatalogEntry],
) -> ParsedInput {
    let decoded = decode_linked_mentions(text);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHistoryText {
    pub text: String,
    pub mentions: Vec<LinkedMention>,
}

pub fn decode_linked_mentions(text: &str) -> DecodedHistoryText {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut mentions = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'['
            && let Some((name, path, end_index)) = parse_linked_tool_mention(text, bytes, index)
            && !is_common_env_var(name)
            && is_tool_path(path)
        {
            out.push('$');
            out.push_str(name);
            mentions.push(LinkedMention {
                mention: name.to_string(),
                path: path.to_string(),
            });
            index = end_index;
            continue;
        }

        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        out.push(ch);
        index += ch.len_utf8();
    }

    DecodedHistoryText {
        text: out,
        mentions,
    }
}

fn parse_linked_tool_mention<'a>(
    text: &'a str,
    text_bytes: &[u8],
    start: usize,
) -> Option<(&'a str, &'a str, usize)> {
    let sigil_index = start + 1;
    if text_bytes.get(sigil_index) != Some(&b'$') {
        return None;
    }

    let name_start = sigil_index + 1;
    let first_name_byte = text_bytes.get(name_start)?;
    if !is_mention_name_char(*first_name_byte) {
        return None;
    }

    let mut name_end = name_start + 1;
    while let Some(next_byte) = text_bytes.get(name_end)
        && is_mention_name_char(*next_byte)
    {
        name_end += 1;
    }

    if text_bytes.get(name_end) != Some(&b']') {
        return None;
    }

    let mut path_start = name_end + 1;
    while let Some(next_byte) = text_bytes.get(path_start)
        && next_byte.is_ascii_whitespace()
    {
        path_start += 1;
    }
    if text_bytes.get(path_start) != Some(&b'(') {
        return None;
    }

    let mut path_end = path_start + 1;
    while let Some(next_byte) = text_bytes.get(path_end)
        && *next_byte != b')'
    {
        path_end += 1;
    }
    if text_bytes.get(path_end) != Some(&b')') {
        return None;
    }

    let path = text[path_start + 1..path_end].trim();
    if path.is_empty() {
        return None;
    }

    let name = &text[name_start..name_end];
    Some((name, path, path_end + 1))
}

fn mention_skill_path(path: &str) -> Option<String> {
    if let Some(stripped) = path.strip_prefix("skill://")
        && !stripped.is_empty()
    {
        return Some(stripped.to_string());
    }
    if path
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
    {
        return Some(path.to_string());
    }
    None
}

fn is_mention_name_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
}

fn is_common_env_var(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "PATH"
            | "HOME"
            | "USER"
            | "SHELL"
            | "PWD"
            | "TMPDIR"
            | "TEMP"
            | "TMP"
            | "LANG"
            | "TERM"
            | "XDG_CONFIG_HOME"
    )
}

fn is_tool_path(path: &str) -> bool {
    path.starts_with("app://")
        || path.starts_with("mcp://")
        || path.starts_with("plugin://")
        || path.starts_with("skill://")
        || path
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
}

#[derive(Debug, Default, Clone)]
struct ToolMentions {
    names: HashSet<String>,
    linked_paths: HashMap<String, String>,
}

fn collect_tool_mentions(text: &str) -> ToolMentions {
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
        if !is_mention_name_char(*first) {
            index += 1;
            continue;
        }
        let mut name_end = name_start + 1;
        while let Some(next) = bytes.get(name_end)
            && is_mention_name_char(*next)
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

fn find_skill_mentions(
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

fn find_app_mentions(
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

fn find_plugin_mentions(text: &str, plugins: &[PluginCatalogEntry]) -> Vec<PluginCatalogEntry> {
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

fn collect_prefixed_tokens(text: &str, sigil: char) -> HashSet<String> {
    let bytes = text.as_bytes();
    let mut tokens = HashSet::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != sigil as u8 {
            index += 1;
            continue;
        }
        if index > 0
            && let Some(previous) = bytes.get(index - 1)
            && !previous.is_ascii_whitespace()
        {
            index += 1;
            continue;
        }

        let start = index + 1;
        let Some(first) = bytes.get(start) else {
            index += 1;
            continue;
        };
        if !is_token_char(*first) {
            index += 1;
            continue;
        }
        let mut end = start + 1;
        while let Some(next) = bytes.get(end)
            && is_token_char(*next)
        {
            end += 1;
        }
        tokens.insert(text[start..end].to_ascii_lowercase());
        index = end;
    }

    tokens
}

fn is_token_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'/')
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::AppCatalogEntry;
    use super::PluginCatalogEntry;
    use super::SkillCatalogEntry;
    use super::build_turn_input;
    use super::decode_linked_mentions;

    #[test]
    fn decode_linked_mentions_restores_visible_tokens() {
        let decoded = decode_linked_mentions(
            "Use [$figma](app://figma-1), [$sample](plugin://sample@test), and [$skill](/tmp/demo/SKILL.md).",
        );
        assert_eq!(decoded.text, "Use $figma, $sample, and $skill.");
        assert_eq!(decoded.mentions.len(), 3);
    }

    #[test]
    fn build_turn_input_includes_images_text_and_mentions() {
        let parsed = build_turn_input(
            "Open [$figma](app://connector_1)",
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
}
