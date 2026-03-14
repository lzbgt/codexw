use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;
use std::time::Instant;

use thiserror::Error;

pub(crate) const DEFAULT_REQUEST_READ_DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub(crate) struct ParsedHttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug, Error)]
pub(crate) enum ReadHttpRequestError {
    #[error("bad request")]
    BadRequest,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub(crate) fn read_http_request(
    stream: &mut TcpStream,
    max_request_bytes: usize,
    request_read_deadline: Duration,
) -> std::result::Result<ParsedHttpRequest, ReadHttpRequestError> {
    let mut buffer = [0_u8; 1024];
    let mut request_bytes = Vec::new();
    let header_deadline = Instant::now() + request_read_deadline;
    let header_end = loop {
        match stream.read(&mut buffer) {
            Ok(0) => return Err(ReadHttpRequestError::BadRequest),
            Ok(read) => {
                request_bytes.extend_from_slice(&buffer[..read]);
                if let Some(index) = request_bytes
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                {
                    break index + 4;
                }
                if request_bytes.len() >= max_request_bytes {
                    return Err(ReadHttpRequestError::BadRequest);
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                if Instant::now() >= header_deadline {
                    return Err(ReadHttpRequestError::Io(err));
                }
                continue;
            }
            Err(err) => return Err(ReadHttpRequestError::Io(err)),
        }
    };

    let request_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let mut lines = request_text.split("\r\n");
    let request_line = lines.next().ok_or(ReadHttpRequestError::BadRequest)?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or(ReadHttpRequestError::BadRequest)?
        .to_string();
    let raw_path = request_parts
        .next()
        .ok_or(ReadHttpRequestError::BadRequest)?;
    if request_parts.next().is_none() {
        return Err(ReadHttpRequestError::BadRequest);
    }

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ReadHttpRequestError::BadRequest);
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    let content_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| ReadHttpRequestError::BadRequest)?
        .unwrap_or(0);

    let mut body = request_bytes[header_end..].to_vec();
    let body_deadline = Instant::now() + request_read_deadline;
    while body.len() < content_length {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                body.extend_from_slice(&buffer[..read]);
                if header_end + body.len() >= max_request_bytes {
                    return Err(ReadHttpRequestError::BadRequest);
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                if Instant::now() >= body_deadline {
                    return Err(ReadHttpRequestError::Io(err));
                }
                continue;
            }
            Err(err) => return Err(ReadHttpRequestError::Io(err)),
        }
    }
    if body.len() < content_length {
        return Err(ReadHttpRequestError::BadRequest);
    }
    body.truncate(content_length);

    Ok(ParsedHttpRequest {
        method,
        path: raw_path.split('?').next().unwrap_or(raw_path).to_string(),
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::read_http_request;
    use std::io::Write;
    use std::net::TcpListener;
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn read_http_request_tolerates_header_fragmentation_across_socket_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_millis(250)))
                .expect("set read timeout");
            let request = read_http_request(&mut stream, 65_536, Duration::from_secs(2))
                .expect("read fragmented request");
            (request.method, request.path, request.headers)
        });

        let mut client = TcpStream::connect(addr).expect("connect client");
        client
            .write_all(b"GET /api/v1/session/sess_test/events HTTP/1.1\r\nHost: localhost\r\n")
            .expect("write first fragment");
        thread::sleep(Duration::from_millis(350));
        client
            .write_all(b"Connection: close\r\n\r\n")
            .expect("write second fragment");

        let (method, path, headers) = server.join().expect("join server");
        assert_eq!(method, "GET");
        assert_eq!(path, "/api/v1/session/sess_test/events");
        assert_eq!(headers.get("host").map(String::as_str), Some("localhost"));
    }

    #[test]
    fn read_http_request_tolerates_body_fragmentation_across_socket_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_millis(250)))
                .expect("set read timeout");
            let request = read_http_request(&mut stream, 65_536, Duration::from_secs(2))
                .expect("read fragmented body");
            (
                request.method,
                request.path,
                String::from_utf8(request.body).expect("decode body"),
            )
        });

        let mut client = TcpStream::connect(addr).expect("connect client");
        client
            .write_all(
                b"POST /api/v1/session/client_event HTTP/1.1\r\nHost: localhost\r\nContent-Length: 17\r\n\r\n{\"event\":\"alpha\"",
            )
            .expect("write first body fragment");
        thread::sleep(Duration::from_millis(350));
        client.write_all(b"}").expect("write second body fragment");

        let (method, path, body) = server.join().expect("join server");
        assert_eq!(method, "POST");
        assert_eq!(path, "/api/v1/session/client_event");
        assert_eq!(body, "{\"event\":\"alpha\"}");
    }
}
