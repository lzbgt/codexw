use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::catalog_backend_views::render_mcp_server_list;
use crate::catalog_feature_views::render_experimental_features_list;
use crate::catalog_file_search::extract_file_search_paths;
use crate::catalog_file_search::render_fuzzy_file_search_results;
use crate::catalog_thread_list::extract_thread_ids;
use crate::catalog_thread_list::render_thread_list;
use crate::output::Output;
use crate::requests::send_list_threads;
use crate::state::AppState;
use crate::status_config::render_config_snapshot;

pub(crate) fn handle_experimental_features_loaded(
    result: &Value,
    output: &mut Output,
) -> Result<()> {
    Ok(output.block_stdout(
        "Experimental features",
        &render_experimental_features_list(result),
    )?)
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
    cwd_filter: Option<&str>,
    allow_fallback_all: bool,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    state.last_listed_thread_ids = extract_thread_ids(result);
    if state.last_listed_thread_ids.is_empty()
        && state.thread_id.is_none()
        && allow_fallback_all
        && search_term.is_none()
        && cwd_filter.is_some()
    {
        output.line_stderr(
            "[session] no exact cwd thread matches; loading recent threads from all workspaces",
        )?;
        return send_list_threads(writer, state, None, None, false);
    }
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
