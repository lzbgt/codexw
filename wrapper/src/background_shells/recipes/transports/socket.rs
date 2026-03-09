use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::time::Duration;

use url::Url;

pub(crate) fn invoke_tcp_recipe(
    endpoint: &str,
    payload: Option<&str>,
    append_newline: bool,
    expect_substring: Option<&str>,
    read_timeout_ms: Option<u64>,
) -> Result<String, String> {
    let (host, port) = parse_tcp_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    let timeout = Duration::from_millis(read_timeout_ms.unwrap_or(500));
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|err| format!("failed to set read timeout for {host}:{port}: {err}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|err| format!("failed to set write timeout for {host}:{port}: {err}"))?;

    let mut payload_line = None;
    if let Some(payload) = payload {
        let mut outbound = payload.as_bytes().to_vec();
        if append_newline {
            outbound.push(b'\n');
        }
        stream
            .write_all(&outbound)
            .map_err(|err| format!("failed to write tcp payload to {host}:{port}: {err}"))?;
        stream
            .flush()
            .map_err(|err| format!("failed to flush tcp payload to {host}:{port}: {err}"))?;
        payload_line = Some(String::from_utf8_lossy(&outbound).into_owned());
    }
    let _ = stream.shutdown(Shutdown::Write);

    let mut response = Vec::new();
    let mut buf = [0_u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(bytes) => response.extend_from_slice(&buf[..bytes]),
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(err) => {
                return Err(format!(
                    "failed to read tcp response from {host}:{port}: {err}"
                ));
            }
        }
    }

    let response_text = String::from_utf8_lossy(&response).into_owned();
    if let Some(expect_substring) = expect_substring
        && !response_text.contains(expect_substring)
    {
        return Err(format!(
            "tcp recipe expected substring `{expect_substring}` but it was not observed.\nResponse:\n{}",
            format_tcp_response(host.as_str(), port, payload_line.as_deref(), &response_text)
        ));
    }

    Ok(format_tcp_response(
        host.as_str(),
        port,
        payload_line.as_deref(),
        &response_text,
    ))
}

pub(crate) fn invoke_redis_recipe(
    endpoint: &str,
    command: &[String],
    expect_substring: Option<&str>,
    read_timeout_ms: Option<u64>,
) -> Result<String, String> {
    let (host, port) = parse_tcp_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    let timeout = Duration::from_millis(read_timeout_ms.unwrap_or(500));
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|err| format!("failed to set read timeout for {host}:{port}: {err}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|err| format!("failed to set write timeout for {host}:{port}: {err}"))?;

    let request = encode_redis_command(command);
    stream
        .write_all(&request)
        .map_err(|err| format!("failed to write redis command to {host}:{port}: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("failed to flush redis command to {host}:{port}: {err}"))?;
    let _ = stream.shutdown(Shutdown::Write);

    let mut response = Vec::new();
    let mut buf = [0_u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(bytes) => response.extend_from_slice(&buf[..bytes]),
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(err) => {
                return Err(format!(
                    "failed to read redis response from {host}:{port}: {err}"
                ));
            }
        }
    }
    let value = parse_redis_response(&response)?;
    let rendered = format_redis_response(host.as_str(), port, command, &value);
    if let Some(expect_substring) = expect_substring
        && !rendered.contains(expect_substring)
    {
        return Err(format!(
            "redis recipe expected substring `{expect_substring}` but it was not observed.\nResponse:\n{rendered}"
        ));
    }
    Ok(rendered)
}

fn parse_tcp_endpoint(endpoint: &str) -> Result<(String, u16), String> {
    if endpoint.contains("://") {
        let url = Url::parse(endpoint)
            .map_err(|err| format!("invalid tcp endpoint `{endpoint}`: {err}"))?;
        if url.scheme() != "tcp" {
            return Err(format!(
                "background shell service endpoint `{endpoint}` uses unsupported scheme `{}` for tcp recipes; use tcp://host:port",
                url.scheme()
            ));
        }
        let host = url
            .host_str()
            .ok_or_else(|| format!("tcp endpoint `{endpoint}` has no host"))?
            .to_string();
        let port = url
            .port()
            .ok_or_else(|| format!("tcp endpoint `{endpoint}` has no explicit port"))?;
        return Ok((host, port));
    }
    let (host, port) = endpoint.rsplit_once(':').ok_or_else(|| {
        format!("tcp endpoint `{endpoint}` must be `host:port` or `tcp://host:port`")
    })?;
    let port = port
        .parse::<u16>()
        .map_err(|err| format!("invalid tcp port in endpoint `{endpoint}`: {err}"))?;
    if host.trim().is_empty() {
        return Err(format!("tcp endpoint `{endpoint}` has an empty host"));
    }
    Ok((host.to_string(), port))
}

fn encode_redis_command(command: &[String]) -> Vec<u8> {
    let mut encoded = format!("*{}\r\n", command.len()).into_bytes();
    for argument in command {
        encoded.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
        encoded.extend_from_slice(argument.as_bytes());
        encoded.extend_from_slice(b"\r\n");
    }
    encoded
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RedisRespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RedisRespValue>>),
}

fn parse_redis_response(bytes: &[u8]) -> Result<RedisRespValue, String> {
    let mut cursor = 0;
    let value = parse_redis_value(bytes, &mut cursor)?;
    Ok(value)
}

fn parse_redis_value(bytes: &[u8], cursor: &mut usize) -> Result<RedisRespValue, String> {
    let marker = *bytes
        .get(*cursor)
        .ok_or_else(|| "redis response was empty".to_string())?;
    *cursor += 1;
    match marker {
        b'+' => Ok(RedisRespValue::SimpleString(read_redis_line(
            bytes, cursor,
        )?)),
        b'-' => Ok(RedisRespValue::Error(read_redis_line(bytes, cursor)?)),
        b':' => {
            let line = read_redis_line(bytes, cursor)?;
            let value = line
                .parse::<i64>()
                .map_err(|err| format!("invalid redis integer response `{line}`: {err}"))?;
            Ok(RedisRespValue::Integer(value))
        }
        b'$' => {
            let line = read_redis_line(bytes, cursor)?;
            let len = line
                .parse::<i64>()
                .map_err(|err| format!("invalid redis bulk length `{line}`: {err}"))?;
            if len == -1 {
                return Ok(RedisRespValue::BulkString(None));
            }
            let len =
                usize::try_from(len).map_err(|_| format!("invalid redis bulk length `{line}`"))?;
            let start = *cursor;
            let end = start + len;
            let payload = bytes
                .get(start..end)
                .ok_or_else(|| "redis bulk string was truncated".to_string())?
                .to_vec();
            *cursor = end;
            consume_redis_crlf(bytes, cursor)?;
            Ok(RedisRespValue::BulkString(Some(payload)))
        }
        b'*' => {
            let line = read_redis_line(bytes, cursor)?;
            let len = line
                .parse::<i64>()
                .map_err(|err| format!("invalid redis array length `{line}`: {err}"))?;
            if len == -1 {
                return Ok(RedisRespValue::Array(None));
            }
            let len =
                usize::try_from(len).map_err(|_| format!("invalid redis array length `{line}`"))?;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(parse_redis_value(bytes, cursor)?);
            }
            Ok(RedisRespValue::Array(Some(items)))
        }
        other => Err(format!(
            "unsupported redis response type byte `{}`",
            other as char
        )),
    }
}

fn read_redis_line(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    let start = *cursor;
    while *cursor + 1 < bytes.len() {
        if bytes[*cursor] == b'\r' && bytes[*cursor + 1] == b'\n' {
            let line = String::from_utf8_lossy(&bytes[start..*cursor]).into_owned();
            *cursor += 2;
            return Ok(line);
        }
        *cursor += 1;
    }
    Err("redis response line was truncated".to_string())
}

fn consume_redis_crlf(bytes: &[u8], cursor: &mut usize) -> Result<(), String> {
    if bytes.get(*cursor) == Some(&b'\r') && bytes.get(*cursor + 1) == Some(&b'\n') {
        *cursor += 2;
        Ok(())
    } else {
        Err("redis bulk string terminator was missing".to_string())
    }
}

fn format_redis_response(
    host: &str,
    port: u16,
    command: &[String],
    value: &RedisRespValue,
) -> String {
    let mut lines = vec![
        format!("Address: {host}:{port}"),
        format!("Command: {}", command.join(" ")),
    ];
    lines.extend(render_redis_value(value, 0));
    lines.join("\n")
}

fn render_redis_value(value: &RedisRespValue, depth: usize) -> Vec<String> {
    let indent = "  ".repeat(depth);
    match value {
        RedisRespValue::SimpleString(text) => vec![
            format!("{indent}Type: simple-string"),
            format!("{indent}Value: {text}"),
        ],
        RedisRespValue::Error(text) => vec![
            format!("{indent}Type: error"),
            format!("{indent}Value: {text}"),
        ],
        RedisRespValue::Integer(value) => vec![
            format!("{indent}Type: integer"),
            format!("{indent}Value: {value}"),
        ],
        RedisRespValue::BulkString(None) => vec![
            format!("{indent}Type: bulk-string"),
            format!("{indent}Value: (nil)"),
        ],
        RedisRespValue::BulkString(Some(bytes)) => vec![
            format!("{indent}Type: bulk-string"),
            format!("{indent}Value: {}", String::from_utf8_lossy(bytes)),
        ],
        RedisRespValue::Array(None) => vec![
            format!("{indent}Type: array"),
            format!("{indent}Value: (nil)"),
        ],
        RedisRespValue::Array(Some(items)) => {
            let mut lines = vec![
                format!("{indent}Type: array"),
                format!("{indent}Length: {}", items.len()),
            ];
            for (index, item) in items.iter().enumerate() {
                lines.push(format!("{indent}Item {index}:"));
                lines.extend(render_redis_value(item, depth + 1));
            }
            lines
        }
    }
}

fn format_tcp_response(host: &str, port: u16, payload: Option<&str>, response: &str) -> String {
    let mut lines = vec![format!("Address: {host}:{port}")];
    if let Some(payload) = payload {
        lines.push("Payload:".to_string());
        lines.extend(payload.lines().map(ToOwned::to_owned));
    }
    if response.is_empty() {
        lines.push("Body: (empty)".to_string());
    } else {
        lines.push("Body:".to_string());
        lines.extend(response.lines().map(ToOwned::to_owned));
    }
    lines.join("\n")
}
