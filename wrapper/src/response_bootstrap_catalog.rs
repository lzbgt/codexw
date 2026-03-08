use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::catalog::parse_apps_list;
use crate::catalog::parse_skills_list;
use crate::catalog_views::extract_file_search_paths;
use crate::catalog_views::extract_thread_ids;
use crate::catalog_views::render_experimental_features_list;
use crate::catalog_views::render_fuzzy_file_search_results;
use crate::catalog_views::render_mcp_server_list;
use crate::catalog_views::render_thread_list;
use crate::collaboration::CollaborationModeAction;
use crate::collaboration::apply_collaboration_mode_action;
use crate::collaboration::extract_collaboration_mode_presets;
use crate::model_session::ModelsAction;
use crate::model_session::apply_models_action;
use crate::output::Output;
use crate::state::AppState;
use crate::status_views::render_config_snapshot;

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

pub(crate) fn handle_experimental_features_loaded(
    result: &Value,
    output: &mut Output,
) -> Result<()> {
    Ok(output.block_stdout(
        "Experimental features",
        &render_experimental_features_list(result),
    )?)
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

pub(crate) fn handle_config_loaded(result: &Value, output: &mut Output) -> Result<()> {
    Ok(output.block_stdout("Config", &render_config_snapshot(result))?)
}

pub(crate) fn handle_mcp_servers_loaded(result: &Value, output: &mut Output) -> Result<()> {
    Ok(output.block_stdout("MCP servers", &render_mcp_server_list(result))?)
}

pub(crate) fn handle_threads_listed(
    result: &Value,
    search_term: Option<&str>,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    state.last_listed_thread_ids = extract_thread_ids(result);
    Ok(output.block_stdout("Threads", &render_thread_list(result, search_term))?)
}

pub(crate) fn handle_fuzzy_file_search(
    result: &Value,
    query: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    let files = result
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    state.last_file_search_paths = extract_file_search_paths(&files);
    let rendered = render_fuzzy_file_search_results(query, files.as_slice());
    Ok(output.block_stdout("File mentions", &rendered)?)
}
