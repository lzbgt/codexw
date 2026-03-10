use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;

use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;
use crate::local_api::LocalApiEvent;
use crate::local_api::SharedEventLog;
use crate::local_api::SharedSnapshot;
use crate::local_api::events::events_since;
use crate::local_api::server::HttpRequest;
use crate::local_api::server::write_response;

use super::json_error_response;

const EVENT_STREAM_POLL_INTERVAL: Duration = Duration::from_millis(100);
const EVENT_STREAM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

pub(in crate::local_api) fn is_event_stream_request(request: &HttpRequest) -> bool {
    request.path.ends_with("/events")
}

pub(in crate::local_api) fn handle_event_stream_request(
    stream: &mut TcpStream,
    request: &HttpRequest,
    snapshot: &SharedSnapshot,
    event_log: &SharedEventLog,
) -> Result<()> {
    if request.method != "GET" {
        write_response(
            stream,
            &json_error_response(405, "method_not_allowed", "unsupported method for route"),
        )?;
        return Ok(());
    }

    let Some(path) = request.path.strip_prefix("/api/v1/session/") else {
        write_response(
            stream,
            &json_error_response(404, "not_found", "unknown route"),
        )?;
        return Ok(());
    };
    let Some(session_id) = path.strip_suffix("/events") else {
        write_response(
            stream,
            &json_error_response(404, "not_found", "unknown route"),
        )?;
        return Ok(());
    };

    let current_snapshot = match snapshot.read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            write_response(
                stream,
                &json_error_response(
                    500,
                    "snapshot_unavailable",
                    "failed to access local API snapshot",
                ),
            )?;
            return Ok(());
        }
    };

    if session_id != current_snapshot.session_id {
        write_response(
            stream,
            &json_error_response(404, "session_not_found", "unknown session id"),
        )?;
        return Ok(());
    }

    let last_event_id = request
        .headers
        .get("last-event-id")
        .and_then(|value| value.parse::<u64>().ok());
    write_event_stream_headers(stream)?;
    write_event_stream_comment(stream, "connected")?;

    let mut sent_event_id = last_event_id;
    let mut last_heartbeat = Instant::now();
    loop {
        let events = events_since(event_log, session_id, sent_event_id);
        for event in events {
            sent_event_id = Some(event.id);
            write_event_stream_event(stream, &event)?;
            last_heartbeat = Instant::now();
        }

        if last_heartbeat.elapsed() >= EVENT_STREAM_HEARTBEAT_INTERVAL {
            write_event_stream_comment(stream, "heartbeat")?;
            last_heartbeat = Instant::now();
        }

        thread::sleep(EVENT_STREAM_POLL_INTERVAL);
    }
}

fn write_event_stream_headers(stream: &mut TcpStream) -> Result<()> {
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nX-Accel-Buffering: no\r\n{}: {}\r\n\r\n",
        HEADER_LOCAL_API_VERSION, CODEXW_LOCAL_API_VERSION,
    );
    stream
        .write_all(headers.as_bytes())
        .context("write local API event stream headers")?;
    stream
        .flush()
        .context("flush local API event stream headers")?;
    Ok(())
}

fn write_event_stream_comment(stream: &mut TcpStream, comment: &str) -> Result<()> {
    stream
        .write_all(format!(": {comment}\n\n").as_bytes())
        .context("write local API event stream comment")?;
    stream
        .flush()
        .context("flush local API event stream comment")?;
    Ok(())
}

fn write_event_stream_event(stream: &mut TcpStream, event: &LocalApiEvent) -> Result<()> {
    let data = serde_json::to_string(&event.data).context("serialize local API SSE event data")?;
    let payload = format!(
        "id: {}\nevent: {}\ndata: {}\n\n",
        event.id, event.event, data
    );
    stream
        .write_all(payload.as_bytes())
        .context("write local API event stream event")?;
    stream
        .flush()
        .context("flush local API event stream event")?;
    Ok(())
}
