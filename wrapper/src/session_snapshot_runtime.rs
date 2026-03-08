use crate::Cli;
use crate::background_terminals::background_terminal_count;
use crate::orchestration_view::orchestration_background_summary;
use crate::orchestration_view::orchestration_guidance_summary;
use crate::orchestration_view::orchestration_runtime_summary;
use crate::session_prompt_status_active::format_elapsed;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::status_account::render_account_summary;
use crate::status_rate_windows::render_rate_limit_lines;
use crate::status_token_usage::render_token_usage_summary;

pub(crate) fn render_status_runtime(_cli: &Cli, state: &AppState) -> Vec<String> {
    let mut lines = Vec::new();

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
    if background_terminal_count(state) > 0 {
        lines.push(format!(
            "background      {}",
            background_terminal_count(state)
        ));
    }
    if let Some(summary) = orchestration_background_summary(state) {
        lines.push(format!("background cls  {summary}"));
    }
    if let Some(summary) = orchestration_runtime_summary(state) {
        lines.push(format!("workers         {summary}"));
    }
    if let Some(guidance) = orchestration_guidance_summary(state) {
        lines.push(format!("next action     {guidance}"));
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

    lines
}
