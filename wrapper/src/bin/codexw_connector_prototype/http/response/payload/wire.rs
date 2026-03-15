use std::io::Write;
use std::net::TcpStream;

use anyhow::Context;

use super::super::super::HttpResponse;

pub(super) fn write_response(
    stream: &mut TcpStream,
    response: &HttpResponse,
) -> anyhow::Result<()> {
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
