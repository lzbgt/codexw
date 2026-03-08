use serde_json::Value;

use crate::input::AppCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::session::extract_models;
use crate::state::get_string;
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

pub(crate) fn render_fuzzy_file_search_results(query: &str, files: &[Value]) -> String {
    if files.is_empty() {
        return format!("No files matched \"{query}\".");
    }
    let mut lines = vec![format!("Query: {query}")];
    for (index, file) in files.iter().take(20).enumerate() {
        let path = get_string(file, &["path"]).unwrap_or("?");
        let score = file
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        lines.push(format!("{:>2}. {}  [score {}]", index + 1, path, score));
    }
    if files.len() > 20 {
        lines.push(format!("...and {} more", files.len() - 20));
    }
    lines.push("Use /mention <n> to insert a match into the prompt.".to_string());
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

pub(crate) fn render_models_list(result: &Value) -> String {
    let models = extract_models(result);
    if models.is_empty() {
        return "No models returned by app-server.".to_string();
    }
    models
        .iter()
        .take(30)
        .map(|model| {
            let default_marker = if model.is_default { " [default]" } else { "" };
            let personality_marker = if model.supports_personality {
                " [supports personality]"
            } else {
                " [personality unsupported]"
            };
            format!(
                "{} ({}){}{}",
                model.display_name, model.id, default_marker, personality_marker
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_mcp_server_list(result: &Value) -> String {
    let entries = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if entries.is_empty() {
        return "No MCP servers returned by app-server.".to_string();
    }
    entries
        .iter()
        .map(|entry| {
            let name = get_string(entry, &["name"]).unwrap_or("?");
            let auth = get_string(entry, &["authStatus"])
                .or_else(|| get_string(entry, &["auth", "status"]))
                .unwrap_or("unknown");
            let tools = entry
                .get("tools")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);
            let resources = entry
                .get("resources")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);
            format!("{name}  [auth {auth}]  [tools {tools}]  [resources {resources}]")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_thread_list(result: &Value, search_term: Option<&str>) -> String {
    let threads = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if threads.is_empty() {
        return match search_term {
            Some(search_term) => format!("No threads matched \"{search_term}\"."),
            None => "No threads found for the current workspace.".to_string(),
        };
    }
    let mut lines = Vec::new();
    if let Some(search_term) = search_term {
        lines.push(format!("Search: {search_term}"));
    }
    lines.extend(threads.iter().enumerate().map(|(index, thread)| {
        let id = get_string(thread, &["id"]).unwrap_or("?");
        let preview = get_string(thread, &["preview"]).unwrap_or("-");
        let status = get_string(thread, &["status", "type"]).unwrap_or("unknown");
        let updated_at = thread
            .get("updatedAt")
            .and_then(Value::as_i64)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "{:>2}. {id}  [{status}]  [updated {updated_at}]  {}",
            index + 1,
            summarize_text(preview)
        )
    }));
    lines.push("Use /resume <n> to resume one of these threads.".to_string());
    lines.join("\n")
}

pub(crate) fn extract_thread_ids(result: &Value) -> Vec<String> {
    result
        .get("data")
        .and_then(Value::as_array)
        .map(|threads| {
            threads
                .iter()
                .filter_map(|thread| get_string(thread, &["id"]).map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn extract_file_search_paths(files: &[Value]) -> Vec<String> {
    files
        .iter()
        .filter_map(|file| get_string(file, &["path"]).map(ToOwned::to_owned))
        .collect()
}
