use crate::Cli;
use crate::collaboration::summarize_active_collaboration_mode;
use crate::model_session::effective_model_entry;
use crate::model_session::summarize_active_personality;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::policy::turn_sandbox_policy;
use crate::session_prompt_status::format_elapsed;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::views::render_account_summary;
use crate::views::render_rate_limit_lines;
use crate::views::render_token_usage_summary;
use crate::views::summarize_sandbox_policy;

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
