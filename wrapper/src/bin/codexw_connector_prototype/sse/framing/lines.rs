use std::net::TcpStream;

use anyhow::Result;

use super::payload::flush_event;

pub(super) fn consume_sse_text(
    text: &str,
    pending_line_fragment: &mut String,
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &super::super::super::Cli,
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
    cli: &super::super::super::Cli,
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
    cli: &super::super::super::Cli,
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
