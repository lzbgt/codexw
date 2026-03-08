use serde_json::Value;

use crate::model_session::extract_models;
use crate::state::get_string;

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
