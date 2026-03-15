use std::io::Write;
use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;

pub(super) fn consume_sse_text(
    text: &str,
    pending_line_fragment: &mut String,
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &super::super::Cli,
) -> Result<()> {
    for line in complete_sse_lines(text, pending_line_fragment) {
        consume_sse_line(
            &line,
            pending_id,
            pending_event,
            pending_data,
            pending_comments,
            client_stream,
            cli,
        )?;
    }
    Ok(())
}

pub(super) fn flush_pending_line_fragment(
    pending_line_fragment: &mut String,
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &super::super::Cli,
) -> Result<()> {
    if pending_line_fragment.is_empty() {
        return Ok(());
    }
    let line = std::mem::take(pending_line_fragment);
    consume_sse_line(
        line.trim_end_matches('\n').trim_end_matches('\r'),
        pending_id,
        pending_event,
        pending_data,
        pending_comments,
        client_stream,
        cli,
    )
}

fn consume_sse_line(
    line: &str,
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &super::super::Cli,
) -> Result<()> {
    if line.is_empty() {
        flush_event(
            pending_id,
            pending_event,
            pending_data,
            pending_comments,
            client_stream,
            cli,
        )?;
        return Ok(());
    }

    if let Some(comment) = line.strip_prefix(':') {
        pending_comments.push(comment.trim_start().to_string());
        return Ok(());
    }
    if let Some(id) = line.strip_prefix("id:") {
        *pending_id = Some(id.trim_start().to_string());
        return Ok(());
    }
    if let Some(event) = line.strip_prefix("event:") {
        *pending_event = Some(event.trim_start().to_string());
        return Ok(());
    }
    if let Some(data) = line.strip_prefix("data:") {
        pending_data.push(data.trim_start().to_string());
    }
    Ok(())
}

pub(super) fn flush_event(
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &super::super::Cli,
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

pub(super) fn complete_sse_lines(text: &str, pending_line_fragment: &mut String) -> Vec<String> {
    let mut completed = Vec::new();
    for segment in text.split_inclusive('\n') {
        pending_line_fragment.push_str(segment);
        if segment.ends_with('\n') {
            completed.push(
                std::mem::take(pending_line_fragment)
                    .trim_end_matches('\n')
                    .trim_end_matches('\r')
                    .to_string(),
            );
        }
    }
    completed
}
