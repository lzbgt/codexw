use std::process::ChildStdin;

use anyhow::Context;
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
use crate::requests::PendingRequest;
use crate::requests::send_initialized;
use crate::requests::send_load_account;
use crate::requests::send_load_apps;
use crate::requests::send_load_collaboration_modes;
use crate::requests::send_load_models;
use crate::requests::send_load_rate_limits;
use crate::requests::send_load_skills;
use crate::runtime::StartMode;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::status_views::render_config_snapshot;

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
            send_initialized(writer)?;
            output.line_stderr("[session] connected")?;
            if let Some(start_mode) = start_after_initialize.take() {
                match start_mode.resume_thread_id {
                    Some(thread_id) => {
                        output.line_stderr(format!("[thread] resume {thread_id}"))?;
                        crate::requests::send_thread_resume(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            thread_id,
                            start_mode.initial_prompt,
                        )?
                    }
                    None => {
                        output.line_stderr("[thread] create")?;
                        crate::requests::send_thread_start(
                            writer,
                            state,
                            cli,
                            resolved_cwd,
                            start_mode.initial_prompt,
                        )?
                    }
                }
            }
            send_load_apps(writer, state)?;
            send_load_skills(writer, state, resolved_cwd)?;
            send_load_models(writer, state, ModelsAction::CacheOnly)?;
            send_load_collaboration_modes(writer, state, CollaborationModeAction::CacheOnly)?;
            send_load_account(writer, state)?;
            send_load_rate_limits(writer, state)?;
        }
        PendingRequest::LoadApps => {
            state.apps = parse_apps_list(result);
        }
        PendingRequest::LoadSkills => {
            state.skills = parse_skills_list(result, resolved_cwd);
        }
        PendingRequest::LoadAccount => {
            state.account_info = result.get("account").cloned();
        }
        PendingRequest::LogoutAccount => {
            state.account_info = None;
            state.rate_limits = None;
            output.line_stderr("[session] logged out")?;
            send_load_account(writer, state)?;
            send_load_rate_limits(writer, state)?;
        }
        PendingRequest::UploadFeedback { classification } => {
            let tracking_thread = get_string(result, &["threadId"]).unwrap_or("-");
            output.line_stderr(format!(
                "[feedback] submitted {} feedback; tracking thread {}",
                summarize_text(classification),
                tracking_thread
            ))?;
        }
        PendingRequest::LoadRateLimits => {
            state.rate_limits = result.get("rateLimits").cloned();
        }
        PendingRequest::LoadModels { action } => {
            apply_models_action(cli, state, action.clone(), result, output)?;
        }
        PendingRequest::LoadExperimentalFeatures => {
            output.block_stdout(
                "Experimental features",
                &render_experimental_features_list(result),
            )?;
        }
        PendingRequest::LoadCollaborationModes { action } => {
            state.collaboration_modes = extract_collaboration_mode_presets(result);
            apply_collaboration_mode_action(state, action.clone(), output)?;
        }
        PendingRequest::LoadConfig => {
            output.block_stdout("Config", &render_config_snapshot(result))?;
        }
        PendingRequest::LoadMcpServers => {
            output.block_stdout("MCP servers", &render_mcp_server_list(result))?;
        }
        PendingRequest::ListThreads { search_term } => {
            state.last_listed_thread_ids = extract_thread_ids(result);
            output.block_stdout(
                "Threads",
                &render_thread_list(result, search_term.as_deref()),
            )?;
        }
        PendingRequest::FuzzyFileSearch { query } => {
            let files = result
                .get("files")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            state.last_file_search_paths = extract_file_search_paths(&files);
            let rendered = render_fuzzy_file_search_results(query, files.as_slice());
            output.block_stdout("File mentions", &rendered)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
