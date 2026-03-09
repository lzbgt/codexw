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
