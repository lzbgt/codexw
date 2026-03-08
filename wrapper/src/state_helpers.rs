use std::collections::HashMap;

use anyhow::Context;
use anyhow::Result;
use base64::Engine;
use serde_json::Value;

use crate::output::Output;
use crate::state::AppState;
use crate::state::ProcessOutputBuffer;

pub(crate) fn thread_id(state: &AppState) -> Result<&str> {
    state
        .thread_id
        .as_deref()
        .context("no active thread; wait for initialization or use :new")
}

pub(crate) fn get_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

pub(crate) fn summarize_text(text: &str) -> String {
    const LIMIT: usize = 120;
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= LIMIT {
        single_line
    } else {
        let truncated = single_line
            .chars()
            .take(LIMIT.saturating_sub(1))
            .collect::<String>();
        format!("{truncated}…")
    }
}

pub(crate) fn emit_status_line(
    _output: &mut Output,
    state: &mut AppState,
    line: String,
) -> Result<()> {
    if state.last_status_line.as_deref() == Some(line.as_str()) {
        return Ok(());
    }
    state.last_status_line = Some(line);
    Ok(())
}

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

pub(crate) fn canonicalize_or_keep(path: &str) -> String {
    std::fs::canonicalize(path)
        .ok()
        .and_then(|value| value.to_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| path.to_string())
}
