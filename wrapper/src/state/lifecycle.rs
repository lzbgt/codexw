#[path = "lifecycle/async_tools.rs"]
mod async_tools;
#[path = "lifecycle/core.rs"]
mod core;
#[path = "lifecycle/observation.rs"]
mod observation;

use crate::rpc::RequestId;

pub(crate) fn request_id_label(id: &RequestId) -> String {
    match id {
        RequestId::Integer(value) => value.to_string(),
        RequestId::String(value) => value.clone(),
    }
}

#[cfg(test)]
fn fallback_async_tool_worker_name(id: &RequestId) -> String {
    format!("codexw-async-tool-worker-{}", request_id_label(id))
}
