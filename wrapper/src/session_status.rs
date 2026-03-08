use std::time::Instant;

use serde_json::Value;

use crate::Cli;
use crate::collaboration::current_collaboration_mode_label;
use crate::collaboration::summarize_active_collaboration_mode;
use crate::model_session::effective_model_entry;
use crate::model_session::summarize_active_personality;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::policy::turn_sandbox_policy;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::views::render_account_summary;
use crate::views::render_rate_limit_lines;
use crate::views::render_token_usage_summary;
use crate::views::summarize_sandbox_policy;
use crate::views::summarize_value;

fn personality_label(personality: &str) -> &str {
    match personality {
        "none" => "None",
        "friendly" => "Friendly",
        "pragmatic" => "Pragmatic",
        _ => personality,
    }
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
