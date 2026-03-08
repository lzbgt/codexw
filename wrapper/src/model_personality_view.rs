use crate::Cli;
use crate::model_catalog::effective_model_entry;
use crate::state::AppState;

pub(crate) fn personality_label(personality: &str) -> &str {
    match personality {
        "none" => "None",
        "friendly" => "Friendly",
        "pragmatic" => "Pragmatic",
        _ => personality,
    }
}

pub(crate) fn summarize_active_personality(state: &AppState) -> String {
    state
        .active_personality
        .as_deref()
        .map(|value| personality_label(value).to_string())
        .unwrap_or_else(|| "default".to_string())
}

pub(crate) fn render_personality_options(cli: &Cli, state: &AppState) -> String {
    let support_line = match effective_model_entry(state, cli) {
        Some(model) if model.supports_personality => format!(
            "current model     {} [supports personality]",
            model.display_name
        ),
        Some(model) => format!(
            "current model     {} [personality unsupported]",
            model.display_name
        ),
        None => "current model     unknown".to_string(),
    };
    [
        format!("current          {}", summarize_active_personality(state)),
        support_line,
        "available choices".to_string(),
        "  - friendly  Warm, collaborative, and helpful.".to_string(),
        "  - pragmatic Concise, task-focused, and direct.".to_string(),
        "  - none      No extra personality instructions.".to_string(),
        "Use /personality <friendly|pragmatic|none|default> to change it.".to_string(),
    ]
    .join("\n")
}
