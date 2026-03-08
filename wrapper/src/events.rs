use anyhow::Result;
use serde_json::Value;
use std::process::ChildStdin;

use crate::Cli;
use crate::notification_item_buffers::handle_buffer_update;
use crate::notification_item_completion::render_item_completed;
use crate::notification_item_status::handle_status_update;
use crate::notification_turn_completed::handle_turn_completed;
use crate::notification_turn_started::handle_turn_started;
use crate::output::Output;
use crate::requests::PendingRequest;
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
use crate::response_error_runtime::handle_runtime_error;
use crate::response_error_session::handle_session_error;
use crate::response_thread_runtime::handle_thread_runtime_response;
use crate::response_thread_session::handle_thread_session_response;
use crate::rpc;
use crate::rpc::IncomingMessage;
use crate::rpc::RpcNotification;
use crate::rpc::RpcResponse;
use crate::runtime_process::StartMode;
use crate::state::AppState;

#[path = "event_requests.rs"]
mod event_requests;
#[path = "notification_realtime.rs"]
mod notification_realtime;

use event_requests::handle_server_request;
#[cfg(test)]
pub(crate) use event_requests::params_auto_approval_result;

pub(crate) fn process_server_line(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    if state.raw_json {
        output.line_stderr(format!("[json] {line}"))?;
    }
    match rpc::parse_line(&line) {
        Ok(IncomingMessage::Response(response)) => handle_response(
            response,
            cli,
            resolved_cwd,
            state,
            output,
            writer,
            start_after_initialize,
        )?,
        Ok(IncomingMessage::Request(request)) => {
            handle_server_request(request, cli, output, writer)?;
        }
        Ok(IncomingMessage::Notification(notification)) => {
            handle_notification(notification, cli, resolved_cwd, state, output, writer)?;
        }
        Err(err) => {
            output.line_stderr(format!("[session] ignored malformed server line: {err}"))?;
        }
    }
    Ok(())
}

fn handle_response(
    response: RpcResponse,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    let pending = state.pending.remove(&response.id);
    if let Some(error) = response.error {
        return handle_response_error(error, pending, state, output);
    }

    let Some(pending) = pending else {
        return Ok(());
    };

    handle_response_success(
        response.result,
        pending,
        cli,
        resolved_cwd,
        state,
        output,
        writer,
        start_after_initialize,
    )
}

fn handle_response_error(
    error: Value,
    pending: Option<PendingRequest>,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    if let Some(pending) = pending.as_ref()
        && (handle_session_error(&error, pending, state, output)?
            || handle_runtime_error(&error, pending, state, output)?)
    {
        return Ok(());
    }
    output.line_stderr(format!(
        "[server-error] {}",
        serde_json::to_string_pretty(&error)?
    ))?;
    Ok(())
}

fn handle_response_success(
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

    if handle_thread_session_response(&pending, &result, cli, resolved_cwd, state, output, writer)?
    {
        return Ok(());
    }

    if handle_thread_runtime_response(&pending, &result, state, output)? {
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

fn handle_notification(
    notification: RpcNotification,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    if notification_realtime::handle_realtime_notification(&notification, cli, state, output)? {
        return Ok(());
    }

    match notification.method.as_str() {
        "skills/changed" => {
            crate::requests::send_load_skills(writer, state, resolved_cwd)?;
            return Ok(());
        }
        "turn/started" => {
            handle_turn_started(&notification, state);
            return Ok(());
        }
        "turn/completed" => {
            handle_turn_completed(&notification, cli, resolved_cwd, state, output, writer)?;
            return Ok(());
        }
        "item/completed" => {
            render_item_completed(&notification.params, state, output)?;
            return Ok(());
        }
        _ => {}
    }

    if handle_buffer_update(
        &notification.method,
        &notification.params,
        cli,
        state,
        output,
    )? || handle_status_update(
        &notification.method,
        &notification.params,
        cli,
        state,
        output,
    )? {
        return Ok(());
    }
    Ok(())
}
