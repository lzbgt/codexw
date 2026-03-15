#[path = "framing/lines.rs"]
mod lines;
#[path = "framing/payload.rs"]
mod payload;

use std::net::TcpStream;

use anyhow::Result;

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
    lines::consume_sse_text(
        text,
        pending_line_fragment,
        pending_id,
        pending_event,
        pending_data,
        pending_comments,
        client_stream,
        cli,
    )
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
    lines::flush_pending_line_fragment(
        pending_line_fragment,
        pending_id,
        pending_event,
        pending_data,
        pending_comments,
        client_stream,
        cli,
    )
}

pub(super) fn flush_event(
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &super::super::Cli,
) -> Result<()> {
    payload::flush_event(
        pending_id,
        pending_event,
        pending_data,
        pending_comments,
        client_stream,
        cli,
    )
}

#[cfg(test)]
pub(super) fn wrap_event_payload(
    data_lines: Vec<String>,
    agent_id: &str,
    deployment_id: &str,
) -> String {
    payload::wrap_event_payload(data_lines, agent_id, deployment_id)
}

#[cfg(test)]
pub(super) fn complete_sse_lines(text: &str, pending_line_fragment: &mut String) -> Vec<String> {
    lines::complete_sse_lines(text, pending_line_fragment)
}
