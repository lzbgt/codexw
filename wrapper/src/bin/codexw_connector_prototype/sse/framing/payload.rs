use std::io::Write;
use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;

pub(super) fn flush_event(
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &super::super::super::Cli,
) -> Result<()> {
    for comment in pending_comments.drain(..) {
        client_stream
            .write_all(format!(": {comment}\n").as_bytes())
            .context("write connector SSE comment")?;
    }
    if pending_id.is_none() && pending_event.is_none() && pending_data.is_empty() {
        client_stream
            .write_all(b"\n")
            .context("write connector SSE separator")?;
        return Ok(());
    }

    if let Some(id) = pending_id.take() {
        client_stream
            .write_all(format!("id: {id}\n").as_bytes())
            .context("write connector SSE id")?;
    }
    if let Some(event) = pending_event.take() {
        client_stream
            .write_all(format!("event: {event}\n").as_bytes())
            .context("write connector SSE event")?;
    }
    let wrapped = wrap_event_payload(
        std::mem::take(pending_data),
        &cli.agent_id,
        &cli.deployment_id,
    );
    client_stream
        .write_all(format!("data: {wrapped}\n\n").as_bytes())
        .context("write connector SSE data")?;
    Ok(())
}

pub(super) fn wrap_event_payload(
    data_lines: Vec<String>,
    agent_id: &str,
    deployment_id: &str,
) -> String {
    let joined = data_lines.join("\n");
    let parsed = serde_json::from_str::<Value>(&joined).unwrap_or_else(|_| Value::String(joined));
    json!({
        "source": "codexw",
        "broker": {
            "agent_id": agent_id,
            "deployment_id": deployment_id,
            "adapter_version": CODEXW_BROKER_ADAPTER_VERSION,
        },
        "data": parsed,
    })
    .to_string()
}
