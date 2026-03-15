use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

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
