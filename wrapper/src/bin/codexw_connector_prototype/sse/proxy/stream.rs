use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpStream;

use anyhow::Context;
use anyhow::Result;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

use super::super::super::Cli;
use super::super::framing::consume_sse_text;
use super::super::framing::flush_event;
use super::super::framing::flush_pending_line_fragment;

pub(super) fn connector_sse_response_head(cli: &Cli, local_api_version: Option<&str>) -> String {
    let mut response_head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nX-Codexw-Agent-Id: {}\r\nX-Codexw-Deployment-Id: {}\r\n{}: {}\r\n",
        cli.agent_id,
        cli.deployment_id,
        HEADER_BROKER_ADAPTER_VERSION,
        CODEXW_BROKER_ADAPTER_VERSION,
    );
    if let Some(local_api_version) = local_api_version {
        response_head.push_str(&format!(
            "{}: {}\r\n",
            HEADER_LOCAL_API_VERSION, local_api_version
        ));
    }
    response_head.push_str("\r\n");
    response_head
}

pub(super) fn bridge_upstream_events(
    client_stream: &mut TcpStream,
    upstream_stream: TcpStream,
    headers: HashMap<String, String>,
    remainder: Vec<u8>,
    cli: &Cli,
) -> Result<()> {
    let response_head = connector_sse_response_head(
        cli,
        headers
            .get("x-codexw-local-api-version")
            .map(String::as_str),
    );
    client_stream
        .write_all(response_head.as_bytes())
        .context("write connector SSE response head")?;

    let mut reader = BufReader::new(upstream_stream);
    let mut pending_id: Option<String> = None;
    let mut pending_event: Option<String> = None;
    let mut pending_data: Vec<String> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    let mut pending_line_fragment = String::new();

    if !remainder.is_empty() {
        consume_sse_text(
            &String::from_utf8_lossy(&remainder),
            &mut pending_line_fragment,
            &mut pending_id,
            &mut pending_event,
            &mut pending_data,
            &mut pending_comments,
            client_stream,
            cli,
        )?;
    }

    let mut line = String::new();
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .context("read upstream SSE line")?;
        if read == 0 {
            flush_pending_line_fragment(
                &mut pending_line_fragment,
                &mut pending_id,
                &mut pending_event,
                &mut pending_data,
                &mut pending_comments,
                client_stream,
                cli,
            )?;
            flush_event(
                &mut pending_id,
                &mut pending_event,
                &mut pending_data,
                &mut pending_comments,
                client_stream,
                cli,
            )?;
            break;
        }
        consume_sse_text(
            &line,
            &mut pending_line_fragment,
            &mut pending_id,
            &mut pending_event,
            &mut pending_data,
            &mut pending_comments,
            client_stream,
            cli,
        )?;
    }

    Ok(())
}
