use std::collections::HashMap;

use base64::Engine;
use serde_json::Value;

use crate::state::ProcessOutputBuffer;

use super::json::get_string;

pub(crate) fn buffer_item_delta(buffers: &mut HashMap<String, String>, params: &Value) {
    let Some(item_id) = get_string(params, &["itemId"]) else {
        return;
    };
    let Some(delta) = get_string(params, &["delta"]) else {
        return;
    };
    buffers
        .entry(item_id.to_string())
        .and_modify(|existing| existing.push_str(delta))
        .or_insert_with(|| delta.to_string());
}

pub(crate) fn buffer_process_delta(
    buffers: &mut HashMap<String, ProcessOutputBuffer>,
    params: &Value,
) {
    let Some(process_id) = get_string(params, &["processId"]) else {
        return;
    };
    let Some(encoded) = get_string(params, &["deltaBase64"]) else {
        return;
    };
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return;
    };
    let text = String::from_utf8_lossy(&decoded);
    let stream = get_string(params, &["stream"]).unwrap_or("stdout");
    let buffer = buffers.entry(process_id.to_string()).or_default();
    match stream {
        "stderr" => buffer.stderr.push_str(&text),
        _ => buffer.stdout.push_str(&text),
    }
}
