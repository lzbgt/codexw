use anyhow::Result;
use serde_json::Value;

use crate::catalog_backend_views::render_mcp_server_list;
use crate::catalog_feature_views::render_experimental_features_list;
use crate::catalog_file_search::extract_file_search_paths;
use crate::catalog_file_search::render_fuzzy_file_search_results;
use crate::catalog_thread_list::thread_list_snapshot;
use crate::output::Output;
use crate::recent_thread_cache::persist_recent_thread_snapshot;
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
    cwd_filter: Option<&str>,
    view: ThreadListView,
    cache_recent_threads: bool,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    let snapshot = thread_list_snapshot(result).filtered_by_cwd(cwd_filter);
    state.last_listed_thread_ids = snapshot.thread_ids();
    if matches!(view, ThreadListView::Agents) {
        state.orchestration.cached_agent_threads = snapshot.agent_thread_summaries();
    }
    if cache_recent_threads {
        let _ = persist_recent_thread_snapshot(state.codex_home_override.as_deref(), &snapshot);
    }
    let title = match view {
        ThreadListView::Threads => "Threads",
        ThreadListView::Agents => "Multi-agents",
    };
    Ok(output.block_stdout(title, &snapshot.render(search_term, view))?)
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
