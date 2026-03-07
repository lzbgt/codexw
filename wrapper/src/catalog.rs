use serde_json::Value;

use crate::input::AppCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::state::get_string;

pub(crate) fn parse_apps_list(result: &Value) -> Vec<AppCatalogEntry> {
    result
        .get("apps")
        .and_then(Value::as_array)
        .or_else(|| result.get("data").and_then(Value::as_array))
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    Some(AppCatalogEntry {
                        id: get_string(entry, &["id"])?.to_string(),
                        name: get_string(entry, &["name"])?.to_string(),
                        slug: app_slug(get_string(entry, &["name"])?),
                        enabled: entry
                            .get("enabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(true),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_skills_list(result: &Value, resolved_cwd: &str) -> Vec<SkillCatalogEntry> {
    result
        .get("roots")
        .and_then(Value::as_array)
        .and_then(|roots| {
            roots
                .iter()
                .find(|entry| get_string(entry, &["cwd"]) == Some(resolved_cwd))
                .or_else(|| roots.first())
        })
        .and_then(|root| root.get("skills").and_then(Value::as_array))
        .map(|skills| {
            skills
                .iter()
                .filter_map(|skill| {
                    Some(SkillCatalogEntry {
                        name: get_string(skill, &["name"])?.to_ascii_lowercase(),
                        path: get_string(skill, &["path"])?.to_string(),
                        enabled: skill
                            .get("enabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(true),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn app_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}
