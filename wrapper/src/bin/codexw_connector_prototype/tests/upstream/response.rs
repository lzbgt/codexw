use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

use crate::upstream::read_upstream_head;
use crate::upstream::read_upstream_response;

fn spawn_response_server(response: &'static [u8]) -> TcpStream {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept");
        socket.write_all(response).expect("write response");
    });
    let stream = TcpStream::connect(addr).expect("connect");
    handle.join().expect("server thread");
    stream
}

#[test]
fn read_upstream_head_parses_status_headers_and_remainder() {
    let mut stream = spawn_response_server(
        b"HTTP/1.1 202 Accepted\r\nContent-Length: 5\r\nX-Test: alpha\r\n\r\nhello",
    );

    let (status, reason, headers, remainder) = read_upstream_head(&mut stream).expect("read head");

    assert_eq!(status, 202);
    assert_eq!(reason, "Accepted");
    assert_eq!(headers.get("content-length").map(String::as_str), Some("5"));
    assert_eq!(headers.get("x-test").map(String::as_str), Some("alpha"));
    assert_eq!(remainder, b"hello");
}

#[test]
fn read_upstream_response_reads_full_body_using_content_length() {
    let stream = spawn_response_server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nContent-Type: application/json\r\n\r\nhello world",
    );

    let response = read_upstream_response(stream).expect("read response");

    assert_eq!(response.status, 200);
    assert_eq!(response.reason, "OK");
    assert_eq!(
        response.headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    assert_eq!(response.body, b"hello world");
}
