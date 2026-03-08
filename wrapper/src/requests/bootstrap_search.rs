use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use super::PendingRequest;
use super::send_json;
use crate::rpc::OutgoingRequest;
use crate::state::AppState;

pub(crate) fn send_list_threads(
    writer: &mut ChildStdin,
    state: &mut AppState,
    cwd_filter: Option<&str>,
    search_term: Option<String>,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::ListThreads {
            search_term: search_term.clone(),
            cwd_filter: cwd_filter.map(ToOwned::to_owned),
        },
    );
    let params = thread_list_params(cwd_filter, search_term.as_deref());
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "thread/list",
            params,
        },
    )
}

pub(crate) fn thread_list_params(cwd_filter: Option<&str>, search_term: Option<&str>) -> Value {
    let mut params = json!({
        "limit": 10,
        "sortKey": "updated_at",
    });
    if let Some(cwd_filter) = cwd_filter {
        params["cwd"] = Value::String(cwd_filter.to_string());
    }
    if let Some(search_term) = search_term {
        params["searchTerm"] = Value::String(search_term.to_string());
    }
    params
}

pub(crate) fn send_fuzzy_file_search(
    writer: &mut ChildStdin,
    state: &mut AppState,
    resolved_cwd: &str,
    query: String,
) -> Result<()> {
    let request_id = state.next_request_id();
    state.pending.insert(
        request_id.clone(),
        PendingRequest::FuzzyFileSearch {
            query: query.clone(),
        },
    );
    send_json(
        writer,
        &OutgoingRequest {
            id: request_id,
            method: "fuzzyFileSearch",
            params: json!({
                "query": query,
                "roots": [resolved_cwd],
            }),
        },
    )
}
