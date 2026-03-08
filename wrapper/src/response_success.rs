use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::runtime_process::StartMode;
use crate::state::AppState;
use serde_json::Value;
use std::process::ChildStdin;

#[path = "response_bootstrap_catalog.rs"]
mod response_bootstrap_catalog;
#[path = "response_bootstrap_init.rs"]
mod response_bootstrap_init;
#[path = "response_threads.rs"]
mod response_threads;

use response_bootstrap_catalog::handle_account_loaded;
use response_bootstrap_catalog::handle_apps_loaded;
use response_bootstrap_catalog::handle_collaboration_modes_loaded;
use response_bootstrap_catalog::handle_config_loaded;
use response_bootstrap_catalog::handle_experimental_features_loaded;
use response_bootstrap_catalog::handle_fuzzy_file_search;
use response_bootstrap_catalog::handle_mcp_servers_loaded;
use response_bootstrap_catalog::handle_models_loaded;
use response_bootstrap_catalog::handle_rate_limits_loaded;
use response_bootstrap_catalog::handle_skills_loaded;
use response_bootstrap_catalog::handle_threads_listed;
use response_bootstrap_init::handle_feedback_success;
use response_bootstrap_init::handle_initialize_success;
use response_bootstrap_init::handle_logout_success;
use response_threads::handle_thread_response_success;

pub(crate) fn handle_response_success(
    result: Value,
    pending: PendingRequest,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if handle_bootstrap_response_success(
        &result,
        &pending,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        start_after_initialize,
    )? {
        return Ok(());
    }

    if handle_thread_response_success(&result, &pending, cli, resolved_cwd, state, output, writer)?
    {
        return Ok(());
    }

    Ok(())
}

fn handle_bootstrap_response_success(
    result: &Value,
    pending: &PendingRequest,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<bool> {
    match pending {
        PendingRequest::Initialize => {
            handle_initialize_success(
                cli,
                resolved_cwd,
                state,
                output,
                writer,
                start_after_initialize,
            )?;
        }
        PendingRequest::LoadApps => {
            handle_apps_loaded(result, state);
        }
        PendingRequest::LoadSkills => {
            handle_skills_loaded(result, resolved_cwd, state);
        }
        PendingRequest::LoadAccount => {
            handle_account_loaded(result, state);
        }
        PendingRequest::LogoutAccount => {
            handle_logout_success(state, output, writer)?;
        }
        PendingRequest::UploadFeedback { classification } => {
            handle_feedback_success(result, classification, output)?;
        }
        PendingRequest::LoadRateLimits => {
            handle_rate_limits_loaded(result, state);
        }
        PendingRequest::LoadModels { action } => {
            handle_models_loaded(cli, result, action.clone(), state, output)?;
        }
        PendingRequest::LoadExperimentalFeatures => {
            handle_experimental_features_loaded(result, output)?;
        }
        PendingRequest::LoadCollaborationModes { action } => {
            handle_collaboration_modes_loaded(result, action.clone(), state, output)?;
        }
        PendingRequest::LoadConfig => {
            handle_config_loaded(result, output)?;
        }
        PendingRequest::LoadMcpServers => {
            handle_mcp_servers_loaded(result, output)?;
        }
        PendingRequest::ListThreads { search_term } => {
            handle_threads_listed(result, search_term.as_deref(), state, output)?;
        }
        PendingRequest::FuzzyFileSearch { query } => {
            handle_fuzzy_file_search(result, query, state, output)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
