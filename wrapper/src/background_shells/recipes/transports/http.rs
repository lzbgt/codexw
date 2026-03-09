use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

use url::Url;

pub(crate) fn invoke_http_recipe(
    endpoint: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
    headers: &[(String, String)],
    expected_status: Option<u16>,
) -> Result<String, String> {
    let base = Url::parse(endpoint)
        .map_err(|err| format!("invalid background shell service endpoint `{endpoint}`: {err}"))?;
    if base.scheme() != "http" {
        return Err(format!(
            "background shell service endpoint `{endpoint}` uses unsupported scheme `{}`; only plain http:// endpoints are currently invokable",
            base.scheme()
        ));
    }
    let request_url = base.join(path).map_err(|err| {
        format!("failed to resolve recipe path `{path}` against endpoint `{endpoint}`: {err}")
    })?;
    let host = request_url
        .host_str()
        .ok_or_else(|| format!("background shell service endpoint `{endpoint}` has no host"))?;
    let port = request_url
        .port_or_known_default()
        .ok_or_else(|| format!("background shell service endpoint `{endpoint}` has no port"))?;
    let request_path = match request_url.query() {
        Some(query) => format!("{}?{query}", request_url.path()),
        None => request_url.path().to_string(),
    };
    let host_header = match request_url.port() {
        Some(port)
            if (request_url.scheme() == "http" && port != 80)
                || (request_url.scheme() == "https" && port != 443) =>
        {
            format!("{host}:{port}")
        }
        _ => host.to_string(),
    };
    let payload = body.unwrap_or_default();
    let mut request =
        format!("{method} {request_path} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if body.is_some() {
        request.push_str(&format!("Content-Length: {}\r\n", payload.len()));
        if !headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("Content-Type"))
        {
            request.push_str("Content-Type: text/plain; charset=utf-8\r\n");
        }
    }
    request.push_str("\r\n");
    if body.is_some() {
        request.push_str(payload);
    }

    let mut stream = TcpStream::connect((host, port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("failed to write request to {host}:{port}: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("failed to flush request to {host}:{port}: {err}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| format!("failed to read response from {host}:{port}: {err}"))?;
    let response = parse_http_response(&String::from_utf8_lossy(&response))?;
    if let Some(expected_status) = expected_status
        && response.status_code != expected_status
    {
        return Err(format!(
            "http recipe expected status {expected_status} but received {}.\nResponse:\n{}",
            response.status_code,
            format_http_response(&response)
        ));
    }
    Ok(format_http_response(&response))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedHttpResponse {
    status_line: String,
    status_code: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn parse_http_response(response: &str) -> Result<ParsedHttpResponse, String> {
    let (head, body) = response
        .split_once("\r\n\r\n")
        .or_else(|| response.split_once("\n\n"))
        .ok_or_else(|| {
            "http recipe returned a malformed response without header separator".to_string()
        })?;
    let mut lines = head.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| "http recipe returned an empty response".to_string())?
        .trim_end_matches('\r')
        .to_string();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("unable to parse HTTP status line `{status_line}`"))?
        .parse::<u16>()
        .map_err(|err| format!("unable to parse HTTP status code from `{status_line}`: {err}"))?;
    let headers = lines
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect::<Vec<_>>();
    Ok(ParsedHttpResponse {
        status_line,
        status_code,
        headers,
        body: body.to_string(),
    })
}

fn format_http_response(response: &ParsedHttpResponse) -> String {
    let mut lines = vec![
        format!("Status: {}", response.status_line),
        format!("Status code: {}", response.status_code),
    ];
    if !response.headers.is_empty() {
        lines.push("Headers:".to_string());
        for (name, value) in &response.headers {
            lines.push(format!("- {name}: {value}"));
        }
    }
    if response.body.is_empty() {
        lines.push("Body: (empty)".to_string());
    } else {
        lines.push("Body:".to_string());
        lines.extend(response.body.lines().map(ToOwned::to_owned));
    }
    lines.join("\n")
}
