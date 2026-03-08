use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::catalog::parse_apps_list;
use crate::catalog::parse_skills_list;
use crate::collaboration::CollaborationModeAction;
use crate::collaboration::apply_collaboration_mode_action;
use crate::collaboration::extract_collaboration_mode_presets;
use crate::model_session::ModelsAction;
use crate::model_session::apply_models_action;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn handle_apps_loaded(result: &Value, state: &mut AppState) {
    state.apps = parse_apps_list(result);
}

pub(crate) fn handle_skills_loaded(result: &Value, resolved_cwd: &str, state: &mut AppState) {
    state.skills = parse_skills_list(result, resolved_cwd);
}

pub(crate) fn handle_account_loaded(result: &Value, state: &mut AppState) {
    state.account_info = result.get("account").cloned();
}

pub(crate) fn handle_rate_limits_loaded(result: &Value, state: &mut AppState) {
    state.rate_limits = result.get("rateLimits").cloned();
}

pub(crate) fn handle_models_loaded(
    cli: &Cli,
    result: &Value,
    action: ModelsAction,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    apply_models_action(cli, state, action, result, output)
}

pub(crate) fn handle_collaboration_modes_loaded(
    result: &Value,
    action: CollaborationModeAction,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    state.collaboration_modes = extract_collaboration_mode_presets(result);
    apply_collaboration_mode_action(state, action, output)
}
