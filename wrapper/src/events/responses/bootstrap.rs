use anyhow::Result;
use serde_json::Value;
use std::process::ChildStdin;

use crate::Cli;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::requests::ThreadListView;
use crate::requests::send_list_agent_threads;
use crate::requests::send_list_threads;
use crate::requests::send_list_threads_with_view;
use crate::response_bootstrap_catalog_state::handle_account_loaded;
use crate::response_bootstrap_catalog_state::handle_apps_loaded;
use crate::response_bootstrap_catalog_state::handle_collaboration_modes_loaded;
use crate::response_bootstrap_catalog_state::handle_models_loaded;
use crate::response_bootstrap_catalog_state::handle_rate_limits_loaded;
use crate::response_bootstrap_catalog_state::handle_skills_loaded;
use crate::response_bootstrap_catalog_views::handle_config_loaded;
use crate::response_bootstrap_catalog_views::handle_experimental_features_loaded;
use crate::response_bootstrap_catalog_views::handle_fuzzy_file_search;
use crate::response_bootstrap_catalog_views::handle_mcp_servers_loaded;
use crate::response_bootstrap_catalog_views::handle_threads_listed;
use crate::response_bootstrap_init::handle_feedback_success;
use crate::response_bootstrap_init::handle_initialize_success;
use crate::response_bootstrap_init::handle_logout_success;
use crate::runtime_process::StartMode;
use crate::state::AppState;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_bootstrap_response_success(
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
        PendingRequest::LoadApps => handle_apps_loaded(result, state),
        PendingRequest::LoadSkills => handle_skills_loaded(result, resolved_cwd, state),
        PendingRequest::LoadAccount => handle_account_loaded(result, state),
        PendingRequest::LogoutAccount => handle_logout_success(state, output, writer)?,
        PendingRequest::UploadFeedback { classification } => {
            handle_feedback_success(result, classification, output)?;
        }
        PendingRequest::LoadRateLimits => handle_rate_limits_loaded(result, state),
        PendingRequest::LoadModels { action } => {
            handle_models_loaded(cli, result, action.clone(), state, output)?;
        }
        PendingRequest::LoadExperimentalFeatures => {
            handle_experimental_features_loaded(result, output)?;
        }
        PendingRequest::WindowsSandboxSetupStart { mode } => {
            output.line_stderr(format!(
                "[session] Windows sandbox setup requested ({mode})"
            ))?;
        }
        PendingRequest::LoadCollaborationModes { action } => {
            handle_collaboration_modes_loaded(result, action.clone(), state, output)?;
        }
        PendingRequest::LoadConfig => handle_config_loaded(result, output)?,
        PendingRequest::LoadMcpServers => handle_mcp_servers_loaded(result, output)?,
        PendingRequest::ListThreads {
            search_term,
            cwd_filter,
            source_kinds,
            view,
        } => {
            if crate::catalog_thread_list::should_fallback_to_all_workspaces(
                result,
                search_term.as_deref(),
                cwd_filter.as_deref(),
            ) {
                output.line_stderr(
                    "[session] no recent threads matched the current workspace; retrying across all workspaces",
                )?;
                if matches!(view, ThreadListView::Agents) && source_kinds.is_none() {
                    send_list_agent_threads(writer, state, None)?;
                } else if source_kinds.is_some() {
                    send_list_threads_with_view(
                        writer,
                        state,
                        None,
                        search_term.clone(),
                        source_kinds.clone(),
                        *view,
                    )?;
                } else {
                    send_list_threads(writer, state, None, search_term.clone())?;
                }
            } else {
                handle_threads_listed(
                    result,
                    search_term.as_deref(),
                    cwd_filter.as_deref(),
                    *view,
                    matches!(view, ThreadListView::Threads) && search_term.is_none(),
                    state,
                    output,
                )?;
            }
        }
        PendingRequest::FuzzyFileSearch { query } => {
            handle_fuzzy_file_search(result, query, state, output)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}
