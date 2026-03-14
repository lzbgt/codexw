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
use serde_json::json;

#[path = "../adapter_contract.rs"]
mod adapter_contract;
#[path = "codexw_connector_prototype/http.rs"]
mod http;
#[path = "../http_request_reader.rs"]
mod http_request_reader;
#[path = "codexw_connector_prototype/routing.rs"]
mod routing;
#[path = "codexw_connector_prototype/sse.rs"]
mod sse;
#[path = "codexw_connector_prototype/tests.rs"]
#[cfg(test)]
mod tests;
#[path = "codexw_connector_prototype/upstream.rs"]
mod upstream;

use http::json_error_response;
use http::json_ok_response;
use http::read_request;
use http::write_response;
use routing::is_allowed_local_proxy_target;
use routing::resolve_proxy_target;
use sse::handle_sse_proxy;
use upstream::ForwardRequestError;
use upstream::forward_request;

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

    let Some(target) = resolve_proxy_target(&request.method, &request.path, &cli.agent_id) else {
        write_response(
            &mut stream,
            &json_error_response(404, "not_found", "unknown connector route", None),
        )?;
        let _ = stream.shutdown(Shutdown::Both);
        return Ok(());
    };

    if target.is_sse && request.method != "GET" {
        write_response(
            &mut stream,
            &json_error_response(
                405,
                "method_not_allowed",
                "unsupported method for SSE route",
                None,
            ),
        )?;
        let _ = stream.shutdown(Shutdown::Both);
        return Ok(());
    }

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

    match forward_request(&request, cli, &target) {
        Ok(upstream) => {
            write_response(&mut stream, &http::from_upstream_response(upstream, cli))?;
        }
        Err(ForwardRequestError::Validation { message, details }) => {
            write_response(
                &mut stream,
                &json_error_response(400, "validation_error", &message, details),
            )?;
        }
        Err(ForwardRequestError::Transport(err)) => {
            write_response(
                &mut stream,
                &json_error_response(
                    502,
                    "upstream_unavailable",
                    "connector could not reach or prepare the local API request",
                    Some(json!({
                        "cause": format!("{err:#}"),
                    })),
                ),
            )?;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}
