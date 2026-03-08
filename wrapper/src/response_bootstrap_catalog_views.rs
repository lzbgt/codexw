use anyhow::Result;
use serde_json::Value;

use crate::catalog_backend_views::render_mcp_server_list;
use crate::catalog_feature_views::render_experimental_features_list;
use crate::catalog_file_search::extract_file_search_paths;
use crate::catalog_file_search::render_fuzzy_file_search_results;
use crate::catalog_thread_list::extract_agent_thread_summaries;
use crate::catalog_thread_list::extract_thread_ids;
use crate::catalog_thread_list::render_thread_list;
use crate::output::Output;
use crate::requests::ThreadListView;
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
    view: ThreadListView,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    let extracted = extract_thread_ids(result);
    state.last_listed_thread_ids = extracted.clone();
    if matches!(view, ThreadListView::Agents) {
        let _ = extracted;
        state.cached_agent_threads = extract_agent_thread_summaries(result);
    }
    let title = match view {
        ThreadListView::Threads => "Threads",
        ThreadListView::Agents => "Multi-agents",
    };
    Ok(output.block_stdout(title, &render_thread_list(result, search_term, view))?)
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
