use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use serde_json::json;
use url::Url;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const READ_TIMEOUT: Duration = Duration::from_millis(500);
const MAX_REQUEST_BYTES: usize = 65536;

#[derive(Parser, Debug, Clone)]
#[command(
    author,
    version,
    about = "Prototype broker-facing connector for the codexw local API"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:0")]
    bind: String,

    #[arg(long, default_value = "http://127.0.0.1:8080")]
    local_api_base: String,

    #[arg(long)]
    local_api_token: Option<String>,

    #[arg(long)]
    connector_token: Option<String>,

    #[arg(long)]
    agent_id: String,

    #[arg(long)]
    deployment_id: String,
}

#[derive(Debug, Clone)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, Clone)]
struct HttpResponse {
    status: u16,
    reason: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Debug, Clone)]
struct UpstreamResponse {
    status: u16,
    reason: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, Clone)]
struct ProxyTarget {
    local_path: String,
    is_sse: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let listener = TcpListener::bind(&cli.bind)
        .with_context(|| format!("bind connector listener on `{}`", cli.bind))?;
    listener
        .set_nonblocking(true)
        .context("set connector listener nonblocking")?;
    let bind_addr = listener
        .local_addr()
        .context("read connector listener address")?;
    eprintln!("codexw connector prototype listening on http://{bind_addr}");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_signal = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        stop_for_signal.store(true, Ordering::Relaxed);
    })
    .context("install ctrl-c handler")?;

    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let cli = cli.clone();
                thread::spawn(move || {
                    let _ = handle_connection(stream, &cli);
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(err) => return Err(err).context("accept connector connection"),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, cli: &Cli) -> Result<()> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set connector read timeout")?;
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(_) => {
            write_response(
                &mut stream,
                &json_error_response(400, "bad_request", "invalid HTTP request", None),
            )?;
            let _ = stream.shutdown(Shutdown::Both);
            return Ok(());
        }
    };

    if request.method == "GET" && request.path == "/healthz" {
        write_response(
            &mut stream,
            &json_ok_response(json!({
                "ok": true,
                "agent_id": cli.agent_id,
                "deployment_id": cli.deployment_id,
            })),
        )?;
        let _ = stream.shutdown(Shutdown::Both);
        return Ok(());
    }

    if let Some(expected_token) = &cli.connector_token {
        match request.headers.get("authorization") {
            Some(value) if value == &format!("Bearer {expected_token}") => {}
            _ => {
                write_response(
                    &mut stream,
                    &json_error_response(
                        401,
                        "unauthorized",
                        "missing or invalid connector bearer token",
                        None,
                    ),
                )?;
                let _ = stream.shutdown(Shutdown::Both);
                return Ok(());
            }
        }
    }

    let Some(target) = strip_proxy_target(&request.path, &cli.agent_id) else {
        write_response(
            &mut stream,
            &json_error_response(404, "not_found", "unknown connector route", None),
        )?;
        let _ = stream.shutdown(Shutdown::Both);
        return Ok(());
    };

    if !is_allowed_local_proxy_target(&request.method, &target.local_path, target.is_sse) {
        write_response(
            &mut stream,
            &json_error_response(
                403,
                "route_not_allowed",
                "connector route is outside the allowed local API surface",
                Some(json!({
                    "method": request.method,
                    "local_path": target.local_path,
                    "is_sse": target.is_sse,
                })),
            ),
        )?;
        let _ = stream.shutdown(Shutdown::Both);
        return Ok(());
    }

    if target.is_sse {
        handle_sse_proxy(stream, &request, cli, &target)?;
        return Ok(());
    }

    let upstream = forward_request(&request, cli, &target.local_path)?;
    write_response(&mut stream, &from_upstream_response(upstream, cli))?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

fn strip_proxy_target(path: &str, agent_id: &str) -> Option<ProxyTarget> {
    let proxy_prefix = format!("/v1/agents/{agent_id}/proxy/");
    if let Some(stripped) = path.strip_prefix(&proxy_prefix) {
        return Some(ProxyTarget {
            local_path: format!("/{}", stripped.trim_start_matches('/')),
            is_sse: false,
        });
    }

    let proxy_sse_prefix = format!("/v1/agents/{agent_id}/proxy_sse/");
    path.strip_prefix(&proxy_sse_prefix)
        .map(|stripped| ProxyTarget {
            local_path: format!("/{}", stripped.trim_start_matches('/')),
            is_sse: true,
        })
}

fn is_allowed_local_proxy_target(method: &str, local_path: &str, is_sse: bool) -> bool {
    let trimmed = local_path.trim_matches('/');
    let segments: Vec<&str> = if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    };

    if is_sse {
        return method == "GET"
            && matches!(segments.as_slice(), ["api", "v1", "session", _, "events"]);
    }

    match method {
        "GET" => matches!(
            segments.as_slice(),
            ["healthz"]
                | ["api", "v1", "session"]
                | ["api", "v1", "session", _]
                | ["api", "v1", "session", _, "transcript"]
                | ["api", "v1", "session", _, "shells"]
                | ["api", "v1", "session", _, "services"]
                | ["api", "v1", "session", _, "capabilities"]
                | ["api", "v1", "session", _, "orchestration", "status"]
                | ["api", "v1", "session", _, "orchestration", "dependencies"]
                | ["api", "v1", "session", _, "orchestration", "workers"]
        ),
        "POST" => matches!(
            segments.as_slice(),
            ["api", "v1", "session", "new"]
                | ["api", "v1", "session", "attach"]
                | ["api", "v1", "session", _, "attachment", "renew"]
                | ["api", "v1", "session", _, "attachment", "release"]
                | ["api", "v1", "session", _, "turn", "start"]
                | ["api", "v1", "session", _, "turn", "interrupt"]
                | ["api", "v1", "session", _, "shells", "start"]
                | ["api", "v1", "session", _, "shells", _, "poll"]
                | ["api", "v1", "session", _, "shells", _, "send"]
                | ["api", "v1", "session", _, "shells", _, "terminate"]
                | ["api", "v1", "session", _, "services", "update"]
                | ["api", "v1", "session", _, "services", _, "provide"]
                | ["api", "v1", "session", _, "services", _, "depend"]
                | ["api", "v1", "session", _, "services", _, "contract"]
                | ["api", "v1", "session", _, "services", _, "relabel"]
                | ["api", "v1", "session", _, "services", _, "attach"]
                | ["api", "v1", "session", _, "services", _, "wait"]
                | ["api", "v1", "session", _, "services", _, "run"]
        ),
        _ => false,
    }
}

fn forward_request(request: &HttpRequest, cli: &Cli, local_path: &str) -> Result<UpstreamResponse> {
    let base = Url::parse(&cli.local_api_base).context("parse local API base URL")?;
    let host = base
        .host_str()
        .context("local API base URL missing host")?
        .to_string();
    let port = base
        .port_or_known_default()
        .context("local API base URL missing port")?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("connect to local API {}:{}", host, port))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .context("set upstream read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("set upstream write timeout")?;

    let path = compose_local_path(&base, local_path);
    write_upstream_request(
        &mut stream,
        &request.method,
        &path,
        request.headers.get("content-type").map(String::as_str),
        cli.local_api_token.as_deref(),
        request.body.as_slice(),
        request.headers.get("last-event-id").map(String::as_str),
    )?;

    read_upstream_response(stream)
}

fn handle_sse_proxy(
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

    let response_head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nX-Codexw-Agent-Id: {}\r\nX-Codexw-Deployment-Id: {}\r\n\r\n",
        cli.agent_id, cli.deployment_id,
    );
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

fn wrap_event_payload(data_lines: Vec<String>, agent_id: &str, deployment_id: &str) -> String {
    let joined = data_lines.join("\n");
    let parsed = serde_json::from_str::<Value>(&joined).unwrap_or_else(|_| Value::String(joined));
    json!({
        "source": "codexw",
        "broker": {
            "agent_id": agent_id,
            "deployment_id": deployment_id,
        },
        "data": parsed,
    })
    .to_string()
}

fn compose_local_path(base: &Url, local_path: &str) -> String {
    let mut prefix = base.path().trim_end_matches('/').to_string();
    if prefix == "/" {
        prefix.clear();
    }
    format!("{prefix}{local_path}")
}

fn write_upstream_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    content_type: Option<&str>,
    auth_token: Option<&str>,
    body: &[u8],
    last_event_id: Option<&str>,
) -> Result<()> {
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(content_type) = content_type {
        request.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    if let Some(auth_token) = auth_token {
        request.push_str(&format!("Authorization: Bearer {auth_token}\r\n"));
    }
    if let Some(last_event_id) = last_event_id {
        request.push_str(&format!("Last-Event-ID: {last_event_id}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .context("write upstream request head")?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .context("write upstream request body")?;
    }
    Ok(())
}

fn read_upstream_response(mut stream: TcpStream) -> Result<UpstreamResponse> {
    let (status, reason, headers, remainder) = read_upstream_head(&mut stream)?;
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = remainder;
    let mut buffer = [0_u8; 4096];
    while body.len() < content_length {
        let read = stream.read(&mut buffer).context("read upstream body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Ok(UpstreamResponse {
        status,
        reason,
        headers,
        body,
    })
}

fn read_error_body(
    status: u16,
    reason: String,
    headers: HashMap<String, String>,
    remainder: Vec<u8>,
    mut stream: TcpStream,
) -> Result<UpstreamResponse> {
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(remainder.len());
    let mut body = remainder;
    let mut buffer = [0_u8; 4096];
    while body.len() < content_length {
        let read = stream
            .read(&mut buffer)
            .context("read upstream error body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Ok(UpstreamResponse {
        status,
        reason,
        headers,
        body,
    })
}

fn read_upstream_head(
    stream: &mut TcpStream,
) -> Result<(u16, String, HashMap<String, String>, Vec<u8>)> {
    let mut buffer = [0_u8; 1024];
    let mut response_bytes = Vec::new();
    let header_end = loop {
        let read = stream.read(&mut buffer).context("read upstream response")?;
        if read == 0 {
            anyhow::bail!("upstream closed before headers");
        }
        response_bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = response_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            break index + 4;
        }
        if response_bytes.len() >= MAX_REQUEST_BYTES {
            anyhow::bail!("upstream response headers too large");
        }
    };
    let header_text = String::from_utf8_lossy(&response_bytes[..header_end]);
    let mut lines = header_text.split("\r\n");
    let status_line = lines.next().context("missing upstream status line")?;
    let mut status_parts = status_line.splitn(3, ' ');
    let _http_version = status_parts.next().context("missing upstream version")?;
    let status = status_parts
        .next()
        .context("missing upstream status code")?
        .parse::<u16>()
        .context("parse upstream status code")?;
    let reason = status_parts.next().unwrap_or("").to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok((
        status,
        reason,
        headers,
        response_bytes[header_end..].to_vec(),
    ))
}

fn from_upstream_response(upstream: UpstreamResponse, cli: &Cli) -> HttpResponse {
    let mut headers = Vec::new();
    if let Some(content_type) = upstream.headers.get("content-type") {
        headers.push(("Content-Type".to_string(), content_type.clone()));
    } else {
        headers.push((
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        ));
    }
    headers.push(("X-Codexw-Agent-Id".to_string(), cli.agent_id.clone()));
    headers.push((
        "X-Codexw-Deployment-Id".to_string(),
        cli.deployment_id.clone(),
    ));
    HttpResponse {
        status: upstream.status,
        reason: Box::leak(upstream.reason.into_boxed_str()),
        headers,
        body: upstream.body,
    }
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut buffer = [0_u8; 1024];
    let mut request_bytes = Vec::new();
    let header_end = loop {
        let read = stream.read(&mut buffer).context("read connector request")?;
        if read == 0 {
            anyhow::bail!("request closed");
        }
        request_bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = request_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            break index + 4;
        }
        if request_bytes.len() >= MAX_REQUEST_BYTES {
            anyhow::bail!("request too large");
        }
    };
    let request_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let mut lines = request_text.split("\r\n");
    let request_line = lines.next().context("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().context("missing method")?.to_string();
    let path = parts
        .next()
        .context("missing path")?
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();
    let _version = parts.next().context("missing version")?;

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut buffer).context("read request body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.reason,
        response.body.len()
    );
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .context("write response head")?;
    if !response.body.is_empty() {
        stream
            .write_all(&response.body)
            .context("write response body")?;
    }
    Ok(())
}

fn json_ok_response(body: Value) -> HttpResponse {
    HttpResponse {
        status: 200,
        reason: "OK",
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: serde_json::to_vec(&body).expect("serialize ok response"),
    }
}

fn json_error_response(
    status: u16,
    code: &str,
    message: &str,
    details: Option<Value>,
) -> HttpResponse {
    let mut error = json!({
        "status": status,
        "code": code,
        "message": message,
    });
    if let Some(details) = details {
        error["details"] = details;
    }
    HttpResponse {
        status,
        reason: match status {
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            405 => "Method Not Allowed",
            502 => "Bad Gateway",
            _ => "Error",
        },
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: serde_json::to_vec(&json!({ "ok": false, "error": error }))
            .expect("serialize error response"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::is_allowed_local_proxy_target;
    use super::strip_proxy_target;
    use super::wrap_event_payload;

    #[test]
    fn strip_proxy_target_maps_http_and_sse_routes() {
        let http = strip_proxy_target(
            "/v1/agents/codexw-lab/proxy/api/v1/session/new",
            "codexw-lab",
        )
        .expect("http route");
        assert_eq!(http.local_path, "/api/v1/session/new");
        assert!(!http.is_sse);

        let sse = strip_proxy_target(
            "/v1/agents/codexw-lab/proxy_sse/api/v1/session/sess_1/events",
            "codexw-lab",
        )
        .expect("sse route");
        assert_eq!(sse.local_path, "/api/v1/session/sess_1/events");
        assert!(sse.is_sse);
    }

    #[test]
    fn strip_proxy_target_rejects_wrong_agent() {
        assert!(
            strip_proxy_target("/v1/agents/other/proxy/api/v1/session/new", "codexw-lab").is_none()
        );
    }

    #[test]
    fn allowlist_accepts_supported_http_routes() {
        assert!(is_allowed_local_proxy_target(
            "POST",
            "/api/v1/session/new",
            false,
        ));
        assert!(is_allowed_local_proxy_target(
            "GET",
            "/api/v1/session/sess_1/orchestration/workers",
            false,
        ));
        assert!(is_allowed_local_proxy_target(
            "POST",
            "/api/v1/session/sess_1/services/bg-1/run",
            false,
        ));
    }

    #[test]
    fn allowlist_accepts_only_session_event_sse_route() {
        assert!(is_allowed_local_proxy_target(
            "GET",
            "/api/v1/session/sess_1/events",
            true,
        ));
        assert!(!is_allowed_local_proxy_target(
            "GET",
            "/api/v1/session/sess_1/transcript",
            true,
        ));
    }

    #[test]
    fn allowlist_rejects_unknown_or_overbroad_proxy_routes() {
        assert!(!is_allowed_local_proxy_target(
            "DELETE",
            "/api/v1/session/sess_1/services/bg-1",
            false,
        ));
        assert!(!is_allowed_local_proxy_target(
            "GET",
            "/api/v1/session/sess_1/internal/debug",
            false,
        ));
        assert!(!is_allowed_local_proxy_target(
            "POST",
            "/api/v1/turn/start",
            false,
        ));
    }

    #[test]
    fn wrap_event_payload_preserves_json_and_adds_broker_metadata() {
        let wrapped = wrap_event_payload(
            vec![r#"{"session_id":"sess_1","value":1}"#.to_string()],
            "codexw-lab",
            "mac-mini-01",
        );
        let json: Value = serde_json::from_str(&wrapped).expect("valid json");
        assert_eq!(json["source"], "codexw");
        assert_eq!(json["broker"]["agent_id"], "codexw-lab");
        assert_eq!(json["broker"]["deployment_id"], "mac-mini-01");
        assert_eq!(json["data"]["session_id"], "sess_1");
        assert_eq!(json["data"]["value"], 1);
    }

    #[test]
    fn wrap_event_payload_falls_back_to_string_for_non_json_data() {
        let wrapped = wrap_event_payload(
            vec!["plain text update".to_string()],
            "codexw-lab",
            "mac-mini-01",
        );
        let json: Value = serde_json::from_str(&wrapped).expect("valid json");
        assert_eq!(json["data"], "plain text update");
    }
}
