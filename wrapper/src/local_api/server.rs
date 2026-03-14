use std::collections::HashMap;
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

use crate::Cli;
use crate::background_shells::BackgroundShellManager;
use crate::http_request_reader::DEFAULT_REQUEST_READ_DEADLINE;
use crate::http_request_reader::ReadHttpRequestError;
use crate::http_request_reader::read_http_request;

use super::SharedCommandQueue;
use super::SharedEventLog;
use super::SharedSnapshot;

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const READ_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_REQUEST_BYTES: usize = 65536;

type RequestReadError = ReadHttpRequestError;

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
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
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
                    event_log,
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
    let request = read_http_request(stream, MAX_REQUEST_BYTES, DEFAULT_REQUEST_READ_DEADLINE)?;
    Ok(HttpRequest {
        method: request.method,
        path: request.path,
        headers: request.headers,
        body: request.body,
    })
}

pub(super) fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> Result<()> {
    let mut headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.reason,
        response.body.len()
    );
    for (name, value) in &response.headers {
        headers.push_str(&format!("{name}: {value}\r\n"));
    }
    headers.push_str("\r\n");
    stream
        .write_all(headers.as_bytes())
        .context("write local API response headers")?;
    stream
        .write_all(&response.body)
        .context("write local API response body")?;
    Ok(())
}
