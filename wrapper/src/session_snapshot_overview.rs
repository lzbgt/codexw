use crate::Cli;
use crate::collaboration::summarize_active_collaboration_mode;
use crate::model_session::effective_model_entry;
use crate::model_session::summarize_active_personality;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::policy::turn_sandbox_policy;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::status_views::summarize_sandbox_policy;

pub(crate) fn render_status_overview(
    cli: &Cli,
    resolved_cwd: &str,
    state: &AppState,
) -> Vec<String> {
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
    lines
}
