use std::collections::HashMap;
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
use thiserror::Error;

use crate::Cli;
use crate::background_shells::BackgroundShellManager;

use super::SharedCommandQueue;
use super::SharedEventLog;
use super::SharedSnapshot;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const READ_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_REQUEST_BYTES: usize = 65536;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use super::routes::route_request;
#[cfg(test)]
pub(crate) use super::routes::route_request_with_manager;

pub(crate) struct LocalApiHandle {
    bind_addr: String,
    stop: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl LocalApiHandle {
    pub(crate) fn bind_addr(&self) -> &str {
        &self.bind_addr
    }

    pub(crate) fn shutdown(mut self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            join_handle
                .join()
                .map_err(|_| anyhow::anyhow!("local API thread panicked"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) reason: &'static str,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug, Error)]
enum RequestReadError {
    #[error("bad request")]
    BadRequest,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub(crate) fn start_local_api(
    cli: &Cli,
    snapshot: SharedSnapshot,
    command_queue: SharedCommandQueue,
    background_shells: BackgroundShellManager,
    event_log: SharedEventLog,
) -> Result<Option<LocalApiHandle>> {
    if !cli.local_api {
        return Ok(None);
    }

    let listener = TcpListener::bind(&cli.local_api_bind)
        .with_context(|| format!("bind local API listener on `{}`", cli.local_api_bind))?;
    listener
        .set_nonblocking(true)
        .context("set local API listener nonblocking")?;
    let bind_addr = listener
        .local_addr()
        .context("read local API listener address")?
        .to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = Arc::clone(&stop);
    let auth_token = cli.local_api_token.clone();

    let join_handle = thread::spawn(move || {
        while !stop_for_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let snapshot = snapshot.clone();
                    let command_queue = command_queue.clone();
                    let background_shells = background_shells.clone();
                    let event_log = event_log.clone();
                    let auth_token = auth_token.clone();
                    thread::spawn(move || {
                        let _ = handle_connection(
                            stream,
                            &snapshot,
                            &command_queue,
                            &background_shells,
                            &event_log,
                            auth_token.as_deref(),
                        );
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(ACCEPT_POLL_INTERVAL);
                }
                Err(_) => break,
            }
        }
    });

    Ok(Some(LocalApiHandle {
        bind_addr,
        stop,
        join_handle: Some(join_handle),
    }))
}

fn handle_connection(
    mut stream: TcpStream,
    snapshot: &SharedSnapshot,
    command_queue: &SharedCommandQueue,
    background_shells: &BackgroundShellManager,
    event_log: &SharedEventLog,
    auth_token: Option<&str>,
) -> Result<()> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set local API read timeout")?;
    let maybe_response = match read_request(&mut stream) {
        Ok(request) => {
            if let Some(response) = super::routes::authorize_request(&request, auth_token) {
                Some(response)
            } else if super::routes::is_event_stream_request(&request) {
                super::routes::handle_event_stream_request(
                    &mut stream,
                    &request,
                    snapshot,
                    event_log,
                )?;
                None
            } else {
                Some(super::routes::route_authorized_request(
                    &request,
                    snapshot,
                    command_queue,
                    background_shells,
                ))
            }
        }
        Err(RequestReadError::BadRequest) => Some(super::routes::json_error_response(
            400,
            "bad_request",
            "invalid HTTP request",
        )),
        Err(RequestReadError::Io(_)) => return Ok(()),
    };
    if let Some(response) = maybe_response {
        write_response(&mut stream, &response)?;
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> std::result::Result<HttpRequest, RequestReadError> {
    let mut buffer = [0_u8; 1024];
    let mut request_bytes = Vec::new();
    let header_end = loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err(RequestReadError::BadRequest);
        }
        request_bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = request_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            break index + 4;
        }
        if request_bytes.len() >= MAX_REQUEST_BYTES {
            return Err(RequestReadError::BadRequest);
        }
    };
    let request_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let mut lines = request_text.split("\r\n");
    let request_line = lines.next().ok_or(RequestReadError::BadRequest)?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().ok_or(RequestReadError::BadRequest)?;
    let raw_path = request_parts.next().ok_or(RequestReadError::BadRequest)?;
    if request_parts.next().is_none() {
        return Err(RequestReadError::BadRequest);
    }

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(RequestReadError::BadRequest);
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    let content_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| RequestReadError::BadRequest)?
        .unwrap_or(0);
    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
        if header_end + body.len() >= MAX_REQUEST_BYTES {
            return Err(RequestReadError::BadRequest);
        }
    }
    if body.len() < content_length {
        return Err(RequestReadError::BadRequest);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method: method.to_string(),
        path: raw_path.split('?').next().unwrap_or(raw_path).to_string(),
        headers,
        body,
    })
}

pub(super) fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<()> {
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        response.reason,
        response.body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .context("write local API response headers")?;
    stream
        .write_all(&response.body)
        .context("write local API response body")?;
    Ok(())
}
