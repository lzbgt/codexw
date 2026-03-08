use serde_json::Value;

use crate::input::AppCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::state::summarize_text;

pub(crate) fn render_experimental_features_list(result: &Value) -> String {
    let mut lines = Vec::new();
    let features = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for feature in features {
        let name = feature.get("name").and_then(Value::as_str).unwrap_or("?");
        let stage = feature
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let enabled = feature
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let default_enabled = feature
            .get("defaultEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let display_name = feature
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or(name);
        let description = feature
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let status = if enabled {
            "enabled"
        } else if default_enabled {
            "default-on"
        } else {
            "disabled"
        };

        lines.push(format!("{display_name}  [{stage}] [{status}]"));
        lines.push(format!("  key: {name}"));
        if !description.is_empty() {
            lines.push(format!("  {description}"));
        }
        if let Some(announcement) = feature.get("announcement").and_then(Value::as_str)
            && !announcement.trim().is_empty()
        {
            lines.push(format!("  note: {}", summarize_text(announcement)));
        }
        lines.push(String::new());
    }

    if lines.is_empty() {
        lines.push("No experimental features were returned by app-server.".to_string());
    } else {
        lines.pop();
    }

    if result.get("nextCursor").and_then(Value::as_str).is_some() {
        lines.push(String::new());
        lines.push("More feature entries are available from app-server.".to_string());
    }

    lines.join("\n")
}

pub(crate) fn render_apps_list(apps: &[AppCatalogEntry]) -> String {
    if apps.is_empty() {
        return "No apps are currently available.".to_string();
    }
    apps.iter()
        .map(|app| {
            format!(
                "{}  ${}  [{}]",
                app.name,
                app.slug,
                if app.enabled { "enabled" } else { "disabled" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_skills_list(skills: &[SkillCatalogEntry]) -> String {
    if skills.is_empty() {
        return "No skills found for the current workspace.".to_string();
    }
    skills
        .iter()
        .map(|skill| {
            format!(
                "{}  {}  [{}]",
                skill.name,
                skill.path,
                if skill.enabled { "enabled" } else { "disabled" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
