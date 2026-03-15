use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use url::Url;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

use super::super::Cli;
use super::super::http::HttpRequest;
use super::super::http::from_upstream_response;
use super::super::http::json_error_response;
use super::super::http::write_response;
use super::super::routing::ProxyTarget;
use super::super::upstream::compose_local_path;
use super::super::upstream::read_error_body;
use super::super::upstream::read_upstream_head;
use super::super::upstream::write_upstream_request;
use super::framing::consume_sse_text;
use super::framing::flush_event;
use super::framing::flush_pending_line_fragment;

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
    let mut pending_line_fragment = String::new();

    if !remainder.is_empty() {
        consume_sse_text(
            &String::from_utf8_lossy(&remainder),
            &mut pending_line_fragment,
            &mut pending_id,
            &mut pending_event,
            &mut pending_data,
            &mut pending_comments,
            &mut client_stream,
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
                &mut client_stream,
                cli,
            )?;
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
        consume_sse_text(
            &line,
            &mut pending_line_fragment,
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
