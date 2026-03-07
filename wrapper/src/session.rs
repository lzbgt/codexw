use std::time::Instant;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::approval_policy;
use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::thread_sandbox_mode;
use crate::turn_sandbox_policy;
use crate::views::render_account_summary;
use crate::views::render_models_list;
use crate::views::render_rate_limit_lines;
use crate::views::render_token_usage_summary;
use crate::views::summarize_sandbox_policy;
use crate::views::summarize_value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CollaborationModePreset {
    pub(crate) name: String,
    pub(crate) mode_kind: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<Option<String>>,
}

impl CollaborationModePreset {
    pub(crate) fn is_plan(&self) -> bool {
        self.mode_kind.as_deref() == Some("plan")
    }

    pub(crate) fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(mode_kind) = self.mode_kind.as_deref() {
            parts.push(format!("mode={mode_kind}"));
        }
        if let Some(model) = self.model.as_deref() {
            parts.push(format!("model={model}"));
        }
        match self.reasoning_effort.as_ref() {
            Some(Some(effort)) => parts.push(format!("effort={effort}")),
            Some(None) => parts.push("effort=default".to_string()),
            None => {}
        }

        if parts.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, parts.join(", "))
        }
    }

    pub(crate) fn turn_start_value(&self) -> Option<Value> {
        let mode = self.mode_kind.as_deref()?;
        let model = self.model.as_deref()?;
        Some(json!({
            "mode": mode,
            "settings": {
                "model": model,
                "reasoning_effort": self.reasoning_effort.clone().flatten(),
                "developer_instructions": Value::Null,
            }
        }))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CollaborationModeAction {
    CacheOnly,
    ShowList,
    TogglePlan,
    SetMode(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelCatalogEntry {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) supports_personality: bool,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum ModelsAction {
    CacheOnly,
    ShowModels,
    ShowPersonality,
    SetPersonality(String),
}

pub(crate) fn current_collaboration_mode_value(state: &AppState) -> Option<Value> {
    state
        .active_collaboration_mode
        .as_ref()
        .and_then(CollaborationModePreset::turn_start_value)
}

fn personality_label(personality: &str) -> &str {
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

pub(crate) fn extract_models(result: &Value) -> Vec<ModelCatalogEntry> {
    result
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let id = get_string(model, &["id"])
                .or_else(|| get_string(model, &["model"]))?
                .to_string();
            let display_name = get_string(model, &["displayName"])
                .unwrap_or(id.as_str())
                .to_string();
            let supports_personality = model
                .get("supportsPersonality")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let is_default = model
                .get("isDefault")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Some(ModelCatalogEntry {
                id,
                display_name,
                supports_personality,
                is_default,
            })
        })
        .collect()
}

fn effective_model_entry<'a>(state: &'a AppState, cli: &Cli) -> Option<&'a ModelCatalogEntry> {
    if let Some(model) = state
        .active_collaboration_mode
        .as_ref()
        .and_then(|preset| preset.model.as_deref())
    {
        return state.models.iter().find(|entry| entry.id == model);
    }
    if let Some(model) = cli.model.as_deref() {
        return state.models.iter().find(|entry| entry.id == model);
    }
    state.models.iter().find(|entry| entry.is_default)
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

pub(crate) fn apply_personality_selection(
    cli: &Cli,
    state: &mut AppState,
    selector: &str,
    output: &mut Output,
) -> Result<()> {
    let normalized = selector.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "default" | "clear") {
        state.active_personality = None;
        output.line_stderr("[session] personality cleared; using backend default")?;
        return Ok(());
    }
    if !matches!(normalized.as_str(), "none" | "friendly" | "pragmatic") {
        output.line_stderr(format!("[session] unknown personality: {selector}"))?;
        output.block_stdout("Personality", &render_personality_options(cli, state))?;
        return Ok(());
    }
    if let Some(model) = effective_model_entry(state, cli)
        && !model.supports_personality
    {
        output.line_stderr(format!(
            "[session] model {} does not support personality overrides",
            model.display_name
        ))?;
        return Ok(());
    }
    state.active_personality = Some(normalized.clone());
    output.line_stderr(format!(
        "[session] personality set to {}",
        personality_label(&normalized)
    ))?;
    Ok(())
}

pub(crate) fn apply_models_action(
    cli: &Cli,
    state: &mut AppState,
    action: ModelsAction,
    result: &Value,
    output: &mut Output,
) -> Result<()> {
    state.models = extract_models(result);
    match action {
        ModelsAction::CacheOnly => {}
        ModelsAction::ShowModels => {
            output.block_stdout("Models", &render_models_list(result))?;
        }
        ModelsAction::ShowPersonality => {
            output.block_stdout("Personality", &render_personality_options(cli, state))?;
        }
        ModelsAction::SetPersonality(selector) => {
            apply_personality_selection(cli, state, &selector, output)?;
        }
    }
    Ok(())
}

fn current_collaboration_mode_label(state: &AppState) -> Option<String> {
    let preset = state.active_collaboration_mode.as_ref()?;
    if preset.is_plan() {
        Some("plan mode".to_string())
    } else {
        Some(format!("collab {}", preset.name))
    }
}

pub(crate) fn summarize_active_collaboration_mode(state: &AppState) -> String {
    state
        .active_collaboration_mode
        .as_ref()
        .map(CollaborationModePreset::summary)
        .unwrap_or_else(|| "default".to_string())
}

pub(crate) fn extract_collaboration_mode_presets(result: &Value) -> Vec<CollaborationModePreset> {
    result
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let mode_kind = item.get("mode").and_then(Value::as_str).map(str::to_string);
            let model = item
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string);
            let reasoning_effort = match item.get("reasoning_effort") {
                Some(Value::String(value)) => Some(Some(value.to_string())),
                Some(Value::Null) => Some(None),
                _ => None,
            };
            Some(CollaborationModePreset {
                name,
                mode_kind,
                model,
                reasoning_effort,
            })
        })
        .collect()
}

fn find_collaboration_mode_by_selector(
    modes: &[CollaborationModePreset],
    selector: &str,
) -> Option<CollaborationModePreset> {
    let normalized = selector.trim().to_ascii_lowercase();
    modes
        .iter()
        .find(|preset| {
            preset
                .mode_kind
                .as_deref()
                .is_some_and(|mode| mode.eq_ignore_ascii_case(&normalized))
                || preset.name.eq_ignore_ascii_case(&normalized)
        })
        .cloned()
}

pub(crate) fn render_collaboration_modes(state: &AppState) -> String {
    let current = summarize_active_collaboration_mode(state);
    if state.collaboration_modes.is_empty() {
        return format!(
            "current         {current}\nno collaboration mode presets available from app-server"
        );
    }

    let mut lines = vec![
        format!("current         {current}"),
        "available presets".to_string(),
    ];
    for (index, preset) in state.collaboration_modes.iter().enumerate() {
        lines.push(format!(" {:>2}. {}", index + 1, preset.summary()));
    }
    lines.push("Use /collab <name|mode> or /plan to switch.".to_string());
    lines.join("\n")
}

pub(crate) fn apply_collaboration_mode_action(
    state: &mut AppState,
    action: CollaborationModeAction,
    output: &mut Output,
) -> Result<()> {
    match action {
        CollaborationModeAction::CacheOnly => {}
        CollaborationModeAction::ShowList => {
            output.block_stdout("Collaboration modes", &render_collaboration_modes(state))?;
        }
        CollaborationModeAction::TogglePlan => {
            if state
                .active_collaboration_mode
                .as_ref()
                .is_some_and(CollaborationModePreset::is_plan)
            {
                state.active_collaboration_mode = None;
                output.line_stderr("[session] collaboration mode cleared; using default mode")?;
            } else if let Some(plan) = state
                .collaboration_modes
                .iter()
                .find(|preset| preset.is_plan())
                .cloned()
            {
                let summary = plan.summary();
                state.active_collaboration_mode = Some(plan);
                output.line_stderr(format!("[session] switched to {summary}"))?;
            } else {
                output.line_stderr(
                    "[session] no plan collaboration preset is available from app-server",
                )?;
            }
        }
        CollaborationModeAction::SetMode(selector) => {
            let normalized = selector.trim().to_ascii_lowercase();
            if matches!(normalized.as_str(), "default" | "off" | "none" | "clear") {
                state.active_collaboration_mode = None;
                output.line_stderr("[session] collaboration mode cleared; using default mode")?;
            } else if let Some(preset) =
                find_collaboration_mode_by_selector(&state.collaboration_modes, &normalized)
            {
                let summary = preset.summary();
                state.active_collaboration_mode = Some(preset);
                output.line_stderr(format!("[session] switched to {summary}"))?;
            } else if state.collaboration_modes.is_empty() {
                output.line_stderr(
                    "[session] no collaboration mode presets are available from app-server",
                )?;
            } else {
                output.line_stderr(format!("[session] unknown collaboration mode: {selector}"))?;
                output.block_stdout("Collaboration modes", &render_collaboration_modes(state))?;
            }
        }
    }

    Ok(())
}

pub(crate) fn render_prompt_status(state: &AppState) -> String {
    let detail = state
        .last_status_line
        .as_deref()
        .filter(|line| !line.trim().is_empty() && *line != "ready");
    if state.active_exec_process_id.is_some() {
        if let Some(detail) = detail {
            format!(
                "{} {} · {}",
                spinner_frame(state.activity_started_at),
                detail,
                format_elapsed(state.activity_started_at),
            )
        } else {
            format!(
                "{} cmd · {}",
                spinner_frame(state.activity_started_at),
                format_elapsed(state.activity_started_at),
            )
        }
    } else if state.turn_running {
        if let Some(detail) = detail {
            format!(
                "{} {} · {}",
                spinner_frame(state.activity_started_at),
                detail,
                format_elapsed(state.activity_started_at),
            )
        } else {
            format!(
                "{} turn {} · {}",
                spinner_frame(state.activity_started_at),
                state.started_turn_count.max(1),
                format_elapsed(state.activity_started_at)
            )
        }
    } else if state.realtime_active {
        format!(
            "{} realtime · {}",
            spinner_frame(state.realtime_started_at),
            format_elapsed(state.realtime_started_at)
        )
    } else {
        match current_collaboration_mode_label(state) {
            Some(label) => match state.active_personality.as_deref() {
                Some(personality) => format!(
                    "ready · {label} · {} · {} turns",
                    personality_label(personality),
                    state.completed_turn_count
                ),
                None => format!("ready · {label} · {} turns", state.completed_turn_count),
            },
            None => match state.active_personality.as_deref() {
                Some(personality) => format!(
                    "ready · {} · {} turns",
                    personality_label(personality),
                    state.completed_turn_count
                ),
                None => format!("ready · {} turns", state.completed_turn_count),
            },
        }
    }
}

fn spinner_frame(started_at: Option<Instant>) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let idx = started_at
        .map(|start| {
            ((Instant::now().saturating_duration_since(start).as_millis() / 100) as usize)
                % FRAMES.len()
        })
        .unwrap_or(0);
    FRAMES[idx]
}

fn format_elapsed(started_at: Option<Instant>) -> String {
    let elapsed = started_at
        .map(|start| Instant::now().saturating_duration_since(start).as_secs())
        .unwrap_or(0);
    if elapsed < 60 {
        format!("{elapsed}s")
    } else {
        format!("{}m{:02}s", elapsed / 60, elapsed % 60)
    }
}

pub(crate) fn render_realtime_status(state: &AppState) -> String {
    let mut lines = vec![format!("active          {}", state.realtime_active)];
    lines.push(format!(
        "session         {}",
        state.realtime_session_id.as_deref().unwrap_or("-")
    ));
    lines.push(format!(
        "prompt          {}",
        summarize_text(state.realtime_prompt.as_deref().unwrap_or("-"))
    ));
    if state.realtime_active {
        lines.push(format!(
            "active time     {}",
            format_elapsed(state.realtime_started_at)
        ));
    }
    if let Some(error) = state.realtime_last_error.as_deref() {
        lines.push(format!("last error      {}", summarize_text(error)));
    }
    lines.push(
        "commands        /realtime start [prompt...] | /realtime send <text> | /realtime stop"
            .to_string(),
    );
    lines.push("audio           output audio deltas are not rendered in codexw".to_string());
    lines.join("\n")
}

pub(crate) fn render_realtime_item(item: &Value) -> String {
    let item_type = get_string(item, &["type"]).unwrap_or("item");
    let item_id = get_string(item, &["id"]).unwrap_or("-");
    let role = get_string(item, &["role"]).unwrap_or("-");
    let body = extract_realtime_text(item).unwrap_or_else(|| summarize_value(item));
    format!(
        "type            {item_type}\nid              {item_id}\nrole            {role}\n\n{}",
        body.trim()
    )
}

pub(crate) fn render_status_snapshot(cli: &Cli, resolved_cwd: &str, state: &AppState) -> String {
    let effective_model_summary = match effective_model_entry(state, cli) {
        Some(model) if model.supports_personality => {
            format!("{} [supports personality]", model.display_name)
        }
        Some(model) => format!("{} [personality unsupported]", model.display_name),
        None => cli.model.as_deref().unwrap_or("default").to_string(),
    };
    let mut lines = vec![
        format!("cwd             {resolved_cwd}"),
        format!(
            "thread          {}",
            state.thread_id.as_deref().unwrap_or("-")
        ),
        format!(
            "turn            {}",
            state.active_turn_id.as_deref().unwrap_or("-")
        ),
        format!(
            "turn count      started={} completed={}",
            state.started_turn_count, state.completed_turn_count
        ),
        format!("running         {}", state.turn_running),
        format!(
            "local command   {}",
            state.active_exec_process_id.as_deref().unwrap_or("-")
        ),
        format!("auto-continue   {}", state.auto_continue),
        format!("approval        {}", approval_policy(cli)),
        format!("sandbox(thread) {}", thread_sandbox_mode(cli)),
        format!(
            "sandbox(turn)   {}",
            summarize_sandbox_policy(&turn_sandbox_policy(cli))
        ),
        format!("model           {}", effective_model_summary),
        format!(
            "provider        {}",
            cli.model_provider.as_deref().unwrap_or("default")
        ),
        format!("personality     {}", summarize_active_personality(state)),
        format!(
            "collaboration   {}",
            summarize_active_collaboration_mode(state)
        ),
        format!("realtime        {}", state.realtime_active),
        format!(
            "objective       {}",
            summarize_text(state.objective.as_deref().unwrap_or("-"))
        ),
        format!(
            "attachments     local={} remote={}",
            state.pending_local_images.len(),
            state.pending_remote_images.len()
        ),
        format!(
            "mentions        apps={} plugins={} skills={}",
            state.apps.iter().filter(|entry| entry.enabled).count(),
            state.plugins.iter().filter(|entry| entry.enabled).count(),
            state.skills.iter().filter(|entry| entry.enabled).count(),
        ),
    ];
    if !state.collaboration_modes.is_empty() {
        lines.push(format!(
            "collab presets  {}",
            state.collaboration_modes.len()
        ));
    }
    if !state.models.is_empty() {
        lines.push(format!("models cached   {}", state.models.len()));
    }
    if state.realtime_active || state.realtime_session_id.is_some() {
        lines.push(format!(
            "realtime id     {}",
            state.realtime_session_id.as_deref().unwrap_or("-")
        ));
    }
    if state.realtime_active {
        lines.push(format!(
            "realtime time   {}",
            format_elapsed(state.realtime_started_at)
        ));
    }
    if let Some(prompt) = state.realtime_prompt.as_deref() {
        lines.push(format!("realtime prompt {}", summarize_text(prompt)));
    }
    if let Some(error) = state.realtime_last_error.as_deref() {
        lines.push(format!("realtime error  {}", summarize_text(error)));
    }

    if let Some(account) = render_account_summary(state.account_info.as_ref()) {
        lines.push(format!("account         {account}"));
    }
    if state.turn_running || state.active_exec_process_id.is_some() {
        lines.push(format!(
            "active time     {}",
            format_elapsed(state.activity_started_at)
        ));
    }
    lines.extend(render_rate_limit_lines(state.rate_limits.as_ref()));
    if let Some(token_usage) = render_token_usage_summary(state.last_token_usage.as_ref()) {
        lines.push(format!("tokens          {token_usage}"));
    }
    if let Some(last_status) = state.last_status_line.as_deref() {
        lines.push(format!("status          {last_status}"));
    }
    if let Some(last_message) = state.last_agent_message.as_deref() {
        lines.push(format!("last reply      {}", summarize_text(last_message)));
    }
    if let Some(diff) = state.last_turn_diff.as_deref() {
        lines.push(format!("diff            {} chars", diff.chars().count()));
    }

    lines.join("\n")
}

fn extract_realtime_text(item: &Value) -> Option<String> {
    if let Some(text) = get_string(item, &["text"]).filter(|text| !text.trim().is_empty()) {
        return Some(text.to_string());
    }
    if let Some(text) = get_string(item, &["transcript"]).filter(|text| !text.trim().is_empty()) {
        return Some(text.to_string());
    }
    item.get("content")
        .and_then(Value::as_array)
        .and_then(|content| {
            let pieces = content
                .iter()
                .filter_map(|part| {
                    get_string(part, &["text"])
                        .or_else(|| get_string(part, &["transcript"]))
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>();
            if pieces.is_empty() {
                None
            } else {
                Some(pieces.join("\n\n"))
            }
        })
}
