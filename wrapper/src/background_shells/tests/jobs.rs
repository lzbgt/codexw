use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::thread;

#[path = "jobs/lifecycle.rs"]
mod lifecycle;
#[path = "jobs/refs.rs"]
mod refs;

#[cfg(unix)]
#[allow(dead_code)]
pub(super) fn interactive_echo_command() -> &'static str {
    "cat"
}

#[cfg(windows)]
#[allow(dead_code)]
pub(super) fn interactive_echo_command() -> &'static str {
    "more"
}

#[allow(dead_code)]
pub(super) fn spawn_test_http_server(
    expected_method: &'static str,
    expected_path: &'static str,
    response_body: &'static str,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        let first_line = request.lines().next().expect("request line");
        assert_eq!(
            first_line,
            format!("{expected_method} {expected_path} HTTP/1.1")
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });
    format!("http://{addr}")
}

#[allow(dead_code)]
pub(super) fn spawn_test_http_server_with_assertions(
    assert_request: impl FnOnce(&str) + Send + 'static,
    response: &'static str,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]).into_owned();
        assert_request(&request);
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });
    format!("http://{addr}")
}
