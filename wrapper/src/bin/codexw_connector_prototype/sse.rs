use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;
use url::Url;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

use super::Cli;
use super::http::HttpRequest;
use super::http::from_upstream_response;
use super::http::json_error_response;
use super::http::write_response;
use super::routing::ProxyTarget;
use super::upstream::compose_local_path;
use super::upstream::read_error_body;
use super::upstream::read_upstream_head;
use super::upstream::write_upstream_request;

pub(super) fn handle_sse_proxy(
    mut client_stream: TcpStream,
    request: &HttpRequest,
    cli: &Cli,
    target: &ProxyTarget,
) -> Result<()> {
    if request.method != "GET" {
        write_response(
            &mut client_stream,
            &json_error_response(
                405,
                "method_not_allowed",
                "unsupported method for SSE route",
                None,
            ),
        )?;
        let _ = client_stream.shutdown(Shutdown::Both);
        return Ok(());
    }

    let base = Url::parse(&cli.local_api_base).context("parse local API base URL")?;
    let host = base
        .host_str()
        .context("local API base URL missing host")?
        .to_string();
    let port = base
        .port_or_known_default()
        .context("local API base URL missing port")?;
    let mut upstream_stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("connect to local API {}:{}", host, port))?;
    upstream_stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .context("set upstream SSE read timeout")?;
    upstream_stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("set upstream SSE write timeout")?;

    let path = compose_local_path(&base, &target.local_path);
    write_upstream_request(
        &mut upstream_stream,
        "GET",
        &path,
        None,
        cli.local_api_token.as_deref(),
        &[],
        request.headers.get("last-event-id").map(String::as_str),
    )?;

    let (status, reason, headers, remainder) = read_upstream_head(&mut upstream_stream)?;
    if status != 200 {
        let upstream = read_error_body(status, reason, headers, remainder, upstream_stream)?;
        write_response(&mut client_stream, &from_upstream_response(upstream, cli))?;
        let _ = client_stream.shutdown(Shutdown::Both);
        return Ok(());
    }

    let mut response_head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nX-Codexw-Agent-Id: {}\r\nX-Codexw-Deployment-Id: {}\r\n{}: {}\r\n",
        cli.agent_id,
        cli.deployment_id,
        HEADER_BROKER_ADAPTER_VERSION,
        CODEXW_BROKER_ADAPTER_VERSION,
    );
    if let Some(local_api_version) = headers.get("x-codexw-local-api-version") {
        response_head.push_str(&format!(
            "{}: {}\r\n",
            HEADER_LOCAL_API_VERSION, local_api_version
        ));
    }
    response_head.push_str("\r\n");
    client_stream
        .write_all(response_head.as_bytes())
        .context("write connector SSE response head")?;

    let mut reader = BufReader::new(upstream_stream);
    let mut pending_id: Option<String> = None;
    let mut pending_event: Option<String> = None;
    let mut pending_data: Vec<String> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();

    if !remainder.is_empty() {
        for line in String::from_utf8_lossy(&remainder).split_inclusive('\n') {
            consume_sse_line(
                line.trim_end_matches('\n').trim_end_matches('\r'),
                &mut pending_id,
                &mut pending_event,
                &mut pending_data,
                &mut pending_comments,
                &mut client_stream,
                cli,
            )?;
        }
    }

    let mut line = String::new();
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .context("read upstream SSE line")?;
        if read == 0 {
            flush_event(
                &mut pending_id,
                &mut pending_event,
                &mut pending_data,
                &mut pending_comments,
                &mut client_stream,
                cli,
            )?;
            break;
        }
        consume_sse_line(
            line.trim_end_matches('\n').trim_end_matches('\r'),
            &mut pending_id,
            &mut pending_event,
            &mut pending_data,
            &mut pending_comments,
            &mut client_stream,
            cli,
        )?;
    }

    let _ = client_stream.shutdown(Shutdown::Both);
    Ok(())
}

fn consume_sse_line(
    line: &str,
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &Cli,
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

fn flush_event(
    pending_id: &mut Option<String>,
    pending_event: &mut Option<String>,
    pending_data: &mut Vec<String>,
    pending_comments: &mut Vec<String>,
    client_stream: &mut TcpStream,
    cli: &Cli,
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
